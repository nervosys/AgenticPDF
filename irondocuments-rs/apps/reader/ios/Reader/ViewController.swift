// SPDX-License-Identifier: AGPL-3.0-or-later
import UIKit
import UniformTypeIdentifiers

/// The iOS shell.
///
/// Built in code rather than storyboards so the whole platform surface reads in
/// one file. Every control calls `Reader.execute` with the same action names the
/// desktop app, the web shell, Android and any agent use — this class
/// implements no document logic of its own.
final class ViewController: UIViewController {

    private let page = PageView()
    private let status = UILabel()
    private let search = UITextField()

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .systemBackground

        let controls = UIStackView(arrangedSubviews: [
            button("Open", #selector(pickDocument)),
            button("‹", #selector(previousPage)),
            button("›", #selector(nextPage)),
            button("−", #selector(zoomOut)),
            button("+", #selector(zoomIn)),
        ])
        controls.distribution = .fillEqually
        controls.spacing = 4

        search.placeholder = "Search…"
        search.borderStyle = .roundedRect
        search.returnKeyType = .search
        search.delegate = self

        let searchRow = UIStackView(arrangedSubviews: [search, button("Save", #selector(save))])
        searchRow.spacing = 8

        status.text = "Open a document to begin."
        status.font = .preferredFont(forTextStyle: .footnote)
        status.textColor = .secondaryLabel
        status.adjustsFontSizeToFitWidth = true

        page.onSwipePage = { [weak self] delta in self?.turn(delta) }
        page.onNoGeometry = { [weak self] in
            self?.status.text = "This format carries no page geometry."
        }

        let root = UIStackView(arrangedSubviews: [controls, searchRow, page, status])
        root.axis = .vertical
        root.spacing = 8
        root.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(root)

        // The safe area is what keeps the controls clear of the notch and the
        // home indicator — the iOS counterpart of Android's window insets, and
        // the same class of bug if it is forgotten.
        let guides = view.safeAreaLayoutGuide
        NSLayoutConstraint.activate([
            root.topAnchor.constraint(equalTo: guides.topAnchor, constant: 8),
            root.leadingAnchor.constraint(equalTo: guides.leadingAnchor, constant: 8),
            root.trailingAnchor.constraint(equalTo: guides.trailingAnchor, constant: -8),
            root.bottomAnchor.constraint(equalTo: guides.bottomAnchor, constant: -8),
        ])
    }

    private func button(_ title: String, _ action: Selector) -> UIButton {
        let button = UIButton(type: .system)
        button.setTitle(title, for: .normal)
        button.addTarget(self, action: action, for: .touchUpInside)
        return button
    }

    /// The document picker, not a path: iOS sandboxes the filesystem, which is
    /// exactly why the core opens from bytes.
    @objc private func pickDocument() {
        let picker = UIDocumentPickerViewController(forOpeningContentTypes: [.data])
        picker.delegate = self
        present(picker, animated: true)
    }

    @objc private func nextPage() { turn(1) }
    @objc private func previousPage() { turn(-1) }
    @objc private func zoomIn() { page.zoom = min(page.zoom * 1.25, 8) }
    @objc private func zoomOut() { page.zoom = max(page.zoom / 1.25, 0.1) }

    private func turn(_ delta: Int) {
        let current = Reader.execute("goto_page", ["page": 1])["page"] as? Int ?? 1
        Reader.execute("goto_page", ["page": current + delta])
        page.setNeedsDisplay()
    }

    @objc private func save() {
        let bytes = Reader.save()
        guard !bytes.isEmpty else { status.text = "Nothing to save."; return }

        let url = FileManager.default.temporaryDirectory.appendingPathComponent("document.adf")
        try? bytes.write(to: url)
        status.text = "Saved \(bytes.count) bytes."
        present(UIActivityViewController(activityItems: [url], applicationActivities: nil),
                animated: true)
    }
}

extension ViewController: UIDocumentPickerDelegate {
    func documentPicker(_ controller: UIDocumentPickerViewController,
                        didPickDocumentsAt urls: [URL]) {
        guard let url = urls.first else { return }

        // Security-scoped access must be started and stopped around the read,
        // or the file is unreadable even though the picker returned it.
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }

        guard let data = try? Data(contentsOf: url) else {
            status.text = "Could not read the selected file."
            return
        }
        let info = Reader.open(data)
        status.text = (info["ok"] as? Bool == true)
            ? "\(info["title"] as? String ?? "Untitled") — \(info["format"] as? String ?? "?"), \(info["pages"] as? Int ?? 0) page(s)"
            : "Could not open: \(info["error"] as? String ?? "unknown error")"
        page.setNeedsDisplay()
    }
}

extension ViewController: UITextFieldDelegate {
    func textFieldShouldReturn(_ textField: UITextField) -> Bool {
        textField.resignFirstResponder()
        let query = (textField.text ?? "").trimmingCharacters(in: .whitespaces)
        guard !query.isEmpty else { return true }

        let hits = Reader.execute("search", ["query": query])["hits"] as? [[String: Any]] ?? []
        status.text = "\(hits.count) result(s)"
        return true
    }
}
