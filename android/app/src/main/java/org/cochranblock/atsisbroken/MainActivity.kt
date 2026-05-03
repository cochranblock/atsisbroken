// SPDX-License-Identifier: Unlicense
package org.cochranblock.atsisbroken

import android.annotation.SuppressLint
import android.os.Bundle
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val webView: WebView = findViewById(R.id.webview)
        webView.settings.apply {
            javaScriptEnabled = true        // required for DOM extraction + autofill
            domStorageEnabled = true
            saveFormData = false             // we are the form-fill layer; the browser is not
            savePassword = false
        }
        webView.webViewClient = WebViewClient()

        // Status banner from Rust core via JNI — proves the bridge is alive.
        val status = Native.nativeStatus()
        webView.loadDataWithBaseURL(
            null,
            """
            <html><body style="font-family:sans-serif;padding:16px">
              <h2>atsisbroken</h2>
              <p>Local-first ATS autopilot. Paste a job URL to begin.</p>
              <pre>${'$'}status</pre>
            </body></html>
            """.trimIndent(),
            "text/html", "utf-8", null
        )
    }
}

object Native {
    init { System.loadLibrary("atsisbroken_android") }
    external fun nativeStatus(): String
    external fun nativeClassify(fieldJson: String): String
}
