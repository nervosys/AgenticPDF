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
            case "stroke_rect", "image":
                // A missing image still shows its frame: quietly omitting
                // content from a document someone is reading is worse than a
                // placeholder.
                context.setStrokeColor(op["op"] as? String == "image"
                                       ? UIColor.gray.cgColor : color(op["color"]))
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
