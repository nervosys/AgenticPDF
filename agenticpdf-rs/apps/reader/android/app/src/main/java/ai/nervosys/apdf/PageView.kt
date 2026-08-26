// SPDX-License-Identifier: AGPL-3.0-or-later
package ai.nervosys.apdf

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.util.AttributeSet
import android.util.Base64
import android.view.GestureDetector
import android.view.MotionEvent
import android.view.View
import org.json.JSONArray
import org.json.JSONObject
import kotlin.math.abs

/** How many decoded masks to hold before starting again. */
private const val MAX_GLYPH_TILES = 4096

/**
 * Draws a page by replaying the recording the Rust core produced.
 *
 * The page-painting decisions — where each glyph sits, which paths fill, how
 * the page is fitted and flipped — were all made in shared Rust. What is left
 * here is a switch over primitives, which is why this file is short and why
 * Android cannot drift from the desktop and web renderings.
 */
class PageView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
) : View(context, attrs) {

    var zoom: Float = 1f
        set(value) {
            field = value.coerceIn(0.1f, 8f)
            invalidate()
        }

    /** Called when a horizontal swipe should turn the page. */
    var onSwipePage: ((Int) -> Unit)? = null

    /** Set when the document has no geometry, so the host can explain itself. */
    var onNoGeometry: (() -> Unit)? = null

    private val fill = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }
    private val stroke = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE }
    private val text = Paint(Paint.ANTI_ALIAS_FLAG)

    private val gestures = GestureDetector(context, object : GestureDetector.SimpleOnGestureListener() {
        override fun onDown(event: MotionEvent) = true

        override fun onFling(
            down: MotionEvent?, up: MotionEvent, vx: Float, vy: Float,
        ): Boolean {
            val dx = up.x - (down?.x ?: return false)
            // Horizontal only, and past a threshold, so a scroll or a stray
            // touch does not turn the page.
            if (abs(dx) > 120 && abs(dx) > abs(up.y - down.y)) {
                onSwipePage?.invoke(if (dx < 0) 1 else -1)
                return true
            }
            return false
        }
    })

    @Suppress("ClickableViewAccessibility") // Handled by the gesture detector.
    override fun onTouchEvent(event: MotionEvent): Boolean =
        gestures.onTouchEvent(event) || super.onTouchEvent(event)

    /**
     * Decoded masks, kept so a redraw does not decode the same letter again.
     *
     * The Android recording carries every mask's pixels on every frame — the
     * native side builds a fresh painter each time and remembers nothing — so
     * this may be cleared whenever it likes. If that ever changes, and the
     * sender starts omitting pixels for masks it believes are held here, then
     * clearing this without telling it will make text silently stop appearing,
     * exactly as the browser shell's comment warns.
     */
    private val glyphTiles = HashMap<String, Bitmap>()

    /**
     * Decode a mask, or return one already decoded.
     *
     * Guarded at every step: a recording that does not carry what this needs
     * falls back to the placeholder frame rather than throwing out of a draw.
     */
    private fun glyphTile(op: JSONObject): Bitmap? {
        val key = op.optString("key")
        if (key.isEmpty()) return null
        glyphTiles[key]?.let { return it }
        val encoded = op.optString("pixels")
        if (encoded.isEmpty()) return null
        val w = op.optInt("iw")
        val h = op.optInt("ih")
        if (w <= 0 || h <= 0) return null
        val bytes = try {
            Base64.decode(encoded, Base64.DEFAULT)
        } catch (_: IllegalArgumentException) {
            return null
        }
        if (bytes.size < w * h * 4) return null
        // The recording is one byte each of R, G, B, A; ARGB_8888 wants them
        // packed into an int the other way round.
        val pixels = IntArray(w * h)
        for (i in 0 until w * h) {
            val r = bytes[i * 4].toInt() and 0xFF
            val g = bytes[i * 4 + 1].toInt() and 0xFF
            val b = bytes[i * 4 + 2].toInt() and 0xFF
            val a = bytes[i * 4 + 3].toInt() and 0xFF
            pixels[i] = (a shl 24) or (r shl 16) or (g shl 8) or b
        }
        val tile = Bitmap.createBitmap(pixels, w, h, Bitmap.Config.ARGB_8888)
        // Every zoom level makes its own masks, so an afternoon of reading
        // would otherwise accumulate them without limit.
        if (glyphTiles.size >= MAX_GLYPH_TILES) {
            glyphTiles.clear()
        }
        glyphTiles[key] = tile
        return tile
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        if (width == 0 || height == 0) return

        val ops: JSONArray = try {
            Reader.renderPage(width.toFloat(), height.toFloat(), zoom)
        } catch (_: Exception) {
            return
        }

        // Balance clips explicitly: the recording pushes and pops, and an
        // unbalanced restore here would corrupt the canvas for the whole view
        // hierarchy, not just this page.
        var clipDepth = 0

        for (index in 0 until ops.length()) {
            val op = ops.optJSONObject(index) ?: continue
            when (op.optString("op")) {
                "fill_rect" -> {
                    fill.color = parseColor(op.optString("color"))
                    canvas.drawRect(
                        op.f("x"), op.f("y"),
                        op.f("x") + op.f("w"), op.f("y") + op.f("h"), fill,
                    )
                }
                "image" -> {
                    // Every glyph on the page arrives here as a mask, so this
                    // is the text path: without it a page reads as a field of
                    // empty rectangles, which is what this drew before.
                    val tile = glyphTile(op)
                    if (tile != null) {
                        canvas.drawBitmap(
                            tile,
                            null,
                            RectF(
                                op.f("x"), op.f("y"),
                                op.f("x") + op.f("w"), op.f("y") + op.f("h"),
                            ),
                            null,
                        )
                    } else {
                        // A mask we were never sent still shows its frame:
                        // silently omitting content from a document someone is
                        // reading is worse than showing a placeholder.
                        stroke.color = Color.GRAY
                        stroke.strokeWidth = 1f
                        canvas.drawRect(
                            op.f("x"), op.f("y"),
                            op.f("x") + op.f("w"), op.f("y") + op.f("h"), stroke,
                        )
                    }
                }
                "stroke_rect" -> {
                    stroke.color = parseColor(op.optString("color"))
                    stroke.strokeWidth = op.optDouble("width", 1.0).toFloat()
                    canvas.drawRect(
                        op.f("x"), op.f("y"),
                        op.f("x") + op.f("w"), op.f("y") + op.f("h"), stroke,
                    )
                }
                "fill_path", "stroke_path" -> {
                    val points = op.optJSONArray("points") ?: continue
                    if (points.length() < 2) continue
                    val path = Path()
                    for (p in 0 until points.length()) {
                        val point = points.optJSONArray(p) ?: continue
                        val x = point.optDouble(0).toFloat()
                        val y = point.optDouble(1).toFloat()
                        if (p == 0) path.moveTo(x, y) else path.lineTo(x, y)
                    }
                    if (op.optString("op") == "fill_path") {
                        path.close()
                        fill.color = parseColor(op.optString("color"))
                        canvas.drawPath(path, fill)
                    } else {
                        stroke.color = parseColor(op.optString("color"))
                        stroke.strokeWidth = op.optDouble("width", 1.0).toFloat()
                        canvas.drawPath(path, stroke)
                    }
                }
                "line" -> {
                    stroke.color = parseColor(op.optString("color"))
                    stroke.strokeWidth = op.optDouble("width", 1.0).toFloat()
                    canvas.drawLine(op.f("x1"), op.f("y1"), op.f("x2"), op.f("y2"), stroke)
                }
                "fill_circle" -> {
                    fill.color = parseColor(op.optString("color"))
                    canvas.drawCircle(op.f("x"), op.f("y"), op.f("r"), fill)
                }
                "stroke_circle" -> {
                    stroke.color = parseColor(op.optString("color"))
                    stroke.strokeWidth = op.optDouble("width", 1.0).toFloat()
                    canvas.drawCircle(op.f("x"), op.f("y"), op.f("r"), stroke)
                }
                "text" -> {
                    text.color = parseColor(op.optString("color"))
                    text.textSize = op.f("size")
                    // The recording anchors text at its top-left to match the
                    // desktop painter; Android draws from the baseline, so the
                    // ascent is added back or every line rides too high.
                    canvas.drawText(
                        op.optString("text"),
                        op.f("x"), op.f("y") - text.fontMetrics.ascent, text,
                    )
                }
                "push_clip" -> {
                    canvas.save()
                    canvas.clipRect(
                        op.f("x"), op.f("y"),
                        op.f("x") + op.f("w"), op.f("y") + op.f("h"),
                    )
                    clipDepth++
                }
                "pop_clip" -> if (clipDepth > 0) { canvas.restore(); clipDepth-- }
                "no_geometry" -> onNoGeometry?.invoke()
            }
        }
        repeat(clipDepth) { canvas.restore() }
    }

    private fun org.json.JSONObject.f(key: String): Float = optDouble(key, 0.0).toFloat()

    /** Parse `rgba(r,g,b,a)` as the recording writes it. */
    private fun parseColor(value: String): Int {
        val parts = value.removePrefix("rgba(").removeSuffix(")").split(",")
        if (parts.size < 4) return Color.BLACK
        return try {
            Color.argb(
                ((parts[3].trim().toFloat()) * 255f).toInt().coerceIn(0, 255),
                parts[0].trim().toInt().coerceIn(0, 255),
                parts[1].trim().toInt().coerceIn(0, 255),
                parts[2].trim().toInt().coerceIn(0, 255),
            )
        } catch (_: NumberFormatException) {
            Color.BLACK
        }
    }
}
