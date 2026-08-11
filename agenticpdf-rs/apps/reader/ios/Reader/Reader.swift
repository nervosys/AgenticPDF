// SPDX-License-Identifier: AGPL-3.0-or-later
import Foundation

/// The Rust core.
///
/// Everything the app knows how to do — open, search, edit, save, paint a page
/// — happens on the other side of these calls, in the same code the desktop,
/// web and Android builds run. This enum is a transport, not a place to put
/// behaviour: a feature implemented here would be one the other platforms and
/// any driving agent do not have.
enum Reader {

    /// Take ownership of a Rust string, decode it, and hand the memory back.
    ///
    /// Every FFI call that returns a string goes through here, so no call site
    /// can leak by forgetting `apdf_string_free`.
    private static func take(_ pointer: UnsafeMutablePointer<CChar>?) -> String {
        guard let pointer else { return "{}" }
        defer { apdf_string_free(pointer) }
        return String(cString: pointer)
    }

    private static func json(_ text: String) -> [String: Any] {
        (try? JSONSerialization.jsonObject(with: Data(text.utf8))) as? [String: Any] ?? [:]
    }

    /// Open a document from bytes. iOS gives security-scoped URLs, not paths.
    static func open(_ data: Data) -> [String: Any] {
        let text = data.withUnsafeBytes { buffer in
            take(apdf_open(buffer.bindMemory(to: UInt8.self).baseAddress, data.count))
        }
        return json(text)
    }

    /// Run an action — the same names the other shells and any agent use.
    @discardableResult
    static func execute(_ action: String, _ params: [String: Any] = [:]) -> [String: Any] {
        let encoded = (try? JSONSerialization.data(withJSONObject: params)) ?? Data("{}".utf8)
        return json(take(apdf_execute(action, String(decoding: encoded, as: UTF8.self))))
    }

    /// Paint the current page into a replayable recording.
    static func renderPage(width: CGFloat, height: CGFloat, zoom: CGFloat) -> [[String: Any]] {
        let text = take(apdf_render_page(Float(width), Float(height), Float(zoom)))
        return ((try? JSONSerialization.jsonObject(with: Data(text.utf8))) as? [[String: Any]]) ?? []
    }

    /// Serialise to ADF. Two calls: ask the size, allocate, fill.
    static func save() -> Data {
        let size = apdf_save(nil, 0)
        guard size > 0 else { return Data() }
        var buffer = [UInt8](repeating: 0, count: size)
        _ = buffer.withUnsafeMutableBufferPointer { apdf_save($0.baseAddress, size) }
        return Data(buffer)
    }

    static var isDirty: Bool { apdf_is_dirty() }

    static func capabilities() -> [String: Any] { json(take(apdf_capabilities())) }
}
