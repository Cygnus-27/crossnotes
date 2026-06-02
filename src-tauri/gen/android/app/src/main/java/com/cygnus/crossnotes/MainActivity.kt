package com.cygnus.crossnotes

import android.content.Context
import android.net.wifi.WifiManager
import android.os.Bundle
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  // Held so the OS delivers multicast packets to us — without it Android
  // silently drops mDNS (224.0.0.251:5353) traffic and discovery never fires.
  private var multicastLock: WifiManager.MulticastLock? = null

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    acquireMulticastLock()
  }

  private fun acquireMulticastLock() {
    val wifiManager =
      applicationContext.getSystemService(Context.WIFI_SERVICE) as? WifiManager ?: return
    multicastLock = wifiManager.createMulticastLock("crossnotes-mdns").apply {
      setReferenceCounted(true)
      acquire()
    }
  }

  override fun onDestroy() {
    multicastLock?.let { if (it.isHeld) it.release() }
    multicastLock = null
    super.onDestroy()
  }
}
