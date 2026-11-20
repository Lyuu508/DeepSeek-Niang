package io.local.deepseekcompanion

import android.content.res.Configuration
import android.graphics.Color
import android.os.Bundle
import android.view.View
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import kotlin.math.max

/** Keep the whole WebView, including dialogs and drawers, inside native safe bounds. */
class MainActivity : TauriActivity() {
    private var webView: WebView? = null
    private var keyboardVisible = false

    override fun onWebViewCreate(webView: WebView) {
        super.onWebViewCreate(webView)
        this.webView = webView
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        val content = findViewById<View>(android.R.id.content)
        val dark = resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
        content.setBackgroundColor(if (dark) Color.rgb(17, 28, 45) else Color.rgb(242, 247, 255))
        ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
            val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
            val keyboard = insets.getInsets(WindowInsetsCompat.Type.ime())
            view.setPadding(bars.left, bars.top, bars.right, max(bars.bottom, keyboard.bottom))
            val visible = insets.isVisible(WindowInsetsCompat.Type.ime())
            if (keyboardVisible != visible) {
                keyboardVisible = visible
                webView?.evaluateJavascript(
                    "window.dispatchEvent(new CustomEvent('companion-ime', {detail: $visible}))",
                    null
                )
            }
            // Native padding owns the insets; do not apply them a second time in WebView CSS.
            WindowInsetsCompat.CONSUMED
        }
        ViewCompat.requestApplyInsets(content)
    }
}
