// SPDX-License-Identifier: AGPL-3.0-or-later
package ai.nervosys.apdf

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.view.Gravity
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import org.json.JSONObject

/**
 * The Android shell.
 *
 * Built in code rather than XML layouts to keep the whole platform surface
 * readable in one file: this is the *only* Android-specific logic, and it is
 * deliberately thin. Every button calls `Reader.execute` with the same action
 * names the desktop app, the web shell and any agent use.
 */
class MainActivity : Activity() {

    private lateinit var page: PageView
    private lateinit var status: TextView
    private lateinit var query: EditText

    private val openDocument = 1001

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val root = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }

        // Android 15 draws targetSdk-35 apps edge to edge, so the content
        // starts at screen y=0 and the status bar sits on top of whatever is
        // first — here, the entire button row, which simply vanished. Insets
        // have to be consumed explicitly. Read from the decor view because this
        // runs before the first layout pass, when the root has none yet.
        root.setOnApplyWindowInsetsListener { view, insets ->
            val bars = insets.getInsets(
                android.view.WindowInsets.Type.systemBars()
                    or android.view.WindowInsets.Type.displayCutout()
            )
            view.setPadding(bars.left, bars.top, bars.right, bars.bottom)
            insets
        }

        // Two rows, not one. Six controls plus a search field do not fit across
        // a phone in portrait — on a 1080px screen the last two were pushed off
        // the edge entirely, which running it on a device is the only way to
        // notice. Each row weights its children so nothing depends on the
        // display width.
        val controls = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
        }
        val even = { LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f) }
        controls.addView(button("Open") { pickDocument() }, even())
        controls.addView(button("‹") { turn(-1) }, even())
        controls.addView(button("›") { turn(1) }, even())
        controls.addView(button("−") { page.zoom /= 1.25f }, even())
        controls.addView(button("+") { page.zoom *= 1.25f }, even())

        val searchRow = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
        }
        query = EditText(this).apply {
            hint = "Search…"
            setOnEditorActionListener { _, _, _ -> search(); true }
        }
        searchRow.addView(query, LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f))
        searchRow.addView(button("Save") { save() })

        page = PageView(this).apply {
            onSwipePage = { delta -> turn(delta) }
            onNoGeometry = { status.text = "This format carries no page geometry." }
        }
        status = TextView(this).apply { text = "Open a document to begin." }

        root.addView(controls, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        root.addView(searchRow, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        root.addView(page, LinearLayout.LayoutParams(MATCH_PARENT, 0, 1f))
        root.addView(status, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        setContentView(root)
    }

    private fun button(label: String, onClick: () -> Unit) =
        Button(this).apply {
            text = label
            setOnClickListener { onClick(); page.invalidate() }
        }

    /**
     * The Storage Access Framework, not a file path: scoped storage means the
     * app never sees a path it could hand to Rust, which is exactly why the
     * core opens from bytes.
     */
    private fun pickDocument() {
        startActivityForResult(
            Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "*/*"
            },
            openDocument,
        )
    }

    override fun onActivityResult(request: Int, result: Int, data: Intent?) {
        super.onActivityResult(request, result, data)
        if (request != openDocument || result != RESULT_OK) return
        val uri: Uri = data?.data ?: return

        val bytes = contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: return
        val info = Reader.open(bytes)
        status.text = if (info.optBoolean("ok"))
            "${info.optString("title")} — ${info.optString("format")}, ${info.optInt("pages")} page(s)"
        else
            "Could not open: ${info.optString("error")}"
        page.invalidate()
    }

    private fun turn(delta: Int) {
        val current = Reader.execute("goto_page", JSONObject().put("page", 1))
            .optInt("page", 1)
        Reader.execute("goto_page", JSONObject().put("page", current + delta))
        page.invalidate()
    }

    private fun search() {
        val text = query.text.toString().trim()
        if (text.isEmpty()) return
        val hits = Reader.execute("search", JSONObject().put("query", text))
            .optJSONArray("hits")
        status.text = "${hits?.length() ?: 0} result(s)"
    }

    private fun save() {
        val bytes = Reader.save()
        if (bytes.isEmpty()) { status.text = "Nothing to save."; return }
        // Written to app-private storage; a share sheet is the natural next
        // step and needs no further Rust.
        openFileOutput("document.adf", MODE_PRIVATE).use { it.write(bytes) }
        status.text = "Saved ${bytes.size} bytes."
        Toast.makeText(this, "Saved document.adf", Toast.LENGTH_SHORT).show()
    }

    // Deprecated in favour of OnBackPressedDispatcher, which lives in AndroidX.
    // This app deliberately has no AndroidX dependency — the shell is meant to
    // stay thin — so the platform method is the right one here.
    @Deprecated("Superseded by OnBackPressedDispatcher in AndroidX")
    override fun onBackPressed() {
        if (Reader.isDirty()) {
            Toast.makeText(this, "Unsaved edits — press again to discard.", Toast.LENGTH_SHORT).show()
            if (!discardArmed) { discardArmed = true; return }
        }
        @Suppress("DEPRECATION")
        super.onBackPressed()
    }

    private var discardArmed = false
}
