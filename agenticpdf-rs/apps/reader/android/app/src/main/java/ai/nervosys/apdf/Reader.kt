// SPDX-License-Identifier: AGPL-3.0-or-later
package ai.nervosys.apdf

import org.json.JSONArray
import org.json.JSONObject

/**
 * The Rust core.
 *
 * Everything the app knows how to do — open, search, edit, save, paint a page —
 * happens on the other side of these calls, in the same code the desktop and
 * web builds run. This object is a transport, not a place to put behaviour: a
 * feature implemented here would be one the other platforms and any driving
 * agent do not have.
 *
 * String and array ownership is JNI's, not ours: the `jni` crate allocates
 * through the JVM, so these results are ordinary Java objects the garbage
 * collector owns and there is nothing to free.
 */
object Reader {
    init {
        System.loadLibrary("apdf_reader")
    }

    private external fun nativeOpen(data: ByteArray): String
    private external fun nativeExecute(action: String, params: String): String
    private external fun nativeRenderPage(width: Float, height: Float, zoom: Float): String
    private external fun nativeSave(): ByteArray
    private external fun nativeIsDirty(): Boolean
    private external fun nativeCapabilities(): String

    /** Open a document from bytes. Android gives content URIs, not paths. */
    fun open(bytes: ByteArray): JSONObject = JSONObject(nativeOpen(bytes))

    /** Run an action — the same names the web shell and any agent use. */
    fun execute(action: String, params: JSONObject = JSONObject()): JSONObject =
        JSONObject(nativeExecute(action, params.toString()))

    /** Paint the current page into a replayable recording. */
    fun renderPage(width: Float, height: Float, zoom: Float): JSONArray =
        JSONArray(nativeRenderPage(width, height, zoom))

    /** Serialise to ADF, for the Storage Access Framework or a share sheet. */
    fun save(): ByteArray = nativeSave()

    fun isDirty(): Boolean = nativeIsDirty()

    fun capabilities(): JSONObject = JSONObject(nativeCapabilities())
}
