// SPDX-License-Identifier: AGPL-3.0-or-later
import UIKit

/// Draws a page by replaying the recording the Rust core produced.
///
/// The page-painting decisions — glyph positions, which paths fill, how the
/// page is fitted and the y axis flipped — were all made in shared Rust. What
/// remains is a switch over primitives, which is why this file is short and why
/// iOS cannot drift from the desktop, web and Android renderings.
final class PageView: UIView {

    var zoom: CGFloat = 1 { didSet { setNeedsDisplay() } }

    /// Called when a horizontal swipe should turn the page.
    var onSwipePage: ((Int) -> Void)?

    /// Called when the document carries no geometry, so the host can say so.
    var onNoGeometry: (() -> Void)?

    override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = .systemBackground

        for (direction, delta) in [(UISwipeGestureRecognizer.Direction.left, 1),
                                   (.right, -1)] {
            let swipe = UISwipeGestureRecognizer(target: self, action: #selector(handleSwipe(_:)))
            swipe.direction = direction
            addGestureRecognizer(swipe)
            swipeDeltas[direction.rawValue] = delta
        }
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }

    private var swipeDeltas: [UInt: Int] = [:]

    @objc private func handleSwipe(_ gesture: UISwipeGestureRecognizer) {
        if let delta = swipeDeltas[gesture.direction.rawValue] { onSwipePage?(delta) }
    }

    /// Decoded masks, kept so a redraw does not decode the same letter again.
    ///
    /// The iOS recording carries every mask's pixels on every frame -- the
    /// native side builds a fresh painter each time and remembers nothing -- so
    /// this may be cleared whenever it likes. If that ever changes, and the
    /// sender starts omitting pixels for masks it believes are held here, then
    /// clearing this without telling it will make text silently stop appearing.
    private var glyphTiles: [String: CGImage] = [:]

    /// How many decoded masks to hold before starting again. Every zoom level
    /// makes its own, so this cannot grow without limit.
    private static let maxGlyphTiles = 4096

    /// Decode a mask, or return one already decoded.
    ///
    /// Guarded at every step: a recording that does not carry what this needs
    /// falls back to the placeholder frame rather than trapping inside a draw.
    private func glyphTile(_ op: [String: Any]) -> CGImage? {
        guard let key = op["key"] as? String, !key.isEmpty else { return nil }
        if let cached = glyphTiles[key] { return cached }
        guard let encoded = op["pixels"] as? String,
              let width = number(op["iw"]).map({ Int($0) }),
              let height = number(op["ih"]).map({ Int($0) }),
              width > 0, height > 0,
              let bytes = Data(base64Encoded: encoded),
              bytes.count >= width * height * 4,
              let provider = CGDataProvider(data: bytes as CFData)
        else { return nil }

        // The recording is straight (not premultiplied) RGBA, one byte each.
        guard let image = CGImage(
            width: width,
            height: height,
            bitsPerComponent: 8,
            bitsPerPixel: 32,
            bytesPerRow: width * 4,
            space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue),
            provider: provider,
            decode: nil,
            shouldInterpolate: true,
            intent: .defaultIntent
        ) else { return nil }

        if glyphTiles.count >= Self.maxGlyphTiles { glyphTiles.removeAll() }
        glyphTiles[key] = image
        return image
    }

    override func draw(_ rect: CGRect) {
        guard let context = UIGraphicsGetCurrentContext(), bounds.width > 0 else { return }

        let ops = Reader.renderPage(width: bounds.width, height: bounds.height, zoom: zoom)

        // Balance clips explicitly. The recording pushes and pops; an
        // unbalanced restore would corrupt the graphics state for everything
        // drawn after this view.
        var clipDepth = 0

        for op in ops {
            switch op["op"] as? String {
            case "fill_rect":
                context.setFillColor(color(op["color"]))
                context.fill(rect(op))
            case "image":
                // Every glyph on the page arrives here as a mask, so this is
                // the text path: without it a page reads as a field of empty
                // rectangles, which is what this drew before.
                if let tile = glyphTile(op) {
                    // Core Graphics draws images up the y axis; the recording
                    // is in screen coordinates, so flip about the destination.
                    context.saveGState()
                    let box = rect(op)
                    context.translateBy(x: 0, y: box.maxY + box.minY)
                    context.scaleBy(x: 1, y: -1)
                    context.draw(tile, in: box)
                    context.restoreGState()
                } else {
                    // A mask we were never sent still shows its frame: quietly
                    // omitting content from a document someone is reading is
                    // worse than a placeholder.
                    context.setStrokeColor(UIColor.gray.cgColor)
                    context.setLineWidth(1)
                    context.stroke(rect(op))
                }
            case "stroke_rect":
                context.setStrokeColor(color(op["color"]))
                context.setLineWidth(number(op["width"]) ?? 1)
                context.stroke(rect(op))
            case "fill_path", "stroke_path":
                guard let points = op["points"] as? [[Double]], points.count >= 2 else { break }
                context.beginPath()
                context.move(to: CGPoint(x: points[0][0], y: points[0][1]))
                for point in points.dropFirst() {
                    context.addLine(to: CGPoint(x: point[0], y: point[1]))
                }
                if op["op"] as? String == "fill_path" {
                    context.closePath()
                    context.setFillColor(color(op["color"]))
                    context.fillPath()
                } else {
                    context.setStrokeColor(color(op["color"]))
                    context.setLineWidth(number(op["width"]) ?? 1)
                    context.strokePath()
                }
            case "line":
                context.beginPath()
                context.move(to: CGPoint(x: number(op["x1"]) ?? 0, y: number(op["y1"]) ?? 0))
                context.addLine(to: CGPoint(x: number(op["x2"]) ?? 0, y: number(op["y2"]) ?? 0))
                context.setStrokeColor(color(op["color"]))
                context.setLineWidth(number(op["width"]) ?? 1)
                context.strokePath()
            case "fill_circle", "stroke_circle":
                let radius = number(op["r"]) ?? 0
                let box = CGRect(x: (number(op["x"]) ?? 0) - radius,
                                 y: (number(op["y"]) ?? 0) - radius,
                                 width: radius * 2, height: radius * 2)
                if op["op"] as? String == "fill_circle" {
                    context.setFillColor(color(op["color"]))
                    context.fillEllipse(in: box)
                } else {
                    context.setStrokeColor(color(op["color"]))
                    context.setLineWidth(number(op["width"]) ?? 1)
                    context.strokeEllipse(in: box)
                }
            case "text":
                let size = number(op["size"]) ?? 12
                let attributes: [NSAttributedString.Key: Any] = [
                    .font: UIFont.systemFont(ofSize: size),
                    .foregroundColor: UIColor(cgColor: color(op["color"])),
                ]
                // The recording anchors text at its top-left, matching every
                // other shell, and UIKit draws from the top-left too — so no
                // baseline adjustment here, unlike Android and Canvas2D.
                (op["text"] as? String ?? "").draw(
                    at: CGPoint(x: number(op["x"]) ?? 0, y: number(op["y"]) ?? 0),
                    withAttributes: attributes)
            case "push_clip":
                context.saveGState()
                context.clip(to: rect(op))
                clipDepth += 1
            case "pop_clip":
                if clipDepth > 0 { context.restoreGState(); clipDepth -= 1 }
            case "no_geometry":
                onNoGeometry?()
            default:
                break
            }
        }
        for _ in 0..<clipDepth { context.restoreGState() }
    }

    private func number(_ value: Any?) -> CGFloat? {
        (value as? NSNumber).map { CGFloat($0.doubleValue) }
    }

    private func rect(_ op: [String: Any]) -> CGRect {
        CGRect(x: number(op["x"]) ?? 0, y: number(op["y"]) ?? 0,
               width: number(op["w"]) ?? 0, height: number(op["h"]) ?? 0)
    }

    /// Parse `rgba(r,g,b,a)` as the recording writes it.
    private func color(_ value: Any?) -> CGColor {
        guard let text = value as? String,
              text.hasPrefix("rgba(") else { return UIColor.black.cgColor }
        let parts = text.dropFirst(5).dropLast()
            .split(separator: ",")
            .map { Double($0.trimmingCharacters(in: .whitespaces)) ?? 0 }
        guard parts.count >= 4 else { return UIColor.black.cgColor }
        return UIColor(red: parts[0] / 255, green: parts[1] / 255,
                       blue: parts[2] / 255, alpha: parts[3]).cgColor
    }
}
