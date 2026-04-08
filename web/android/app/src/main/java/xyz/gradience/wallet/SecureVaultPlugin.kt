package xyz.gradience.wallet

import android.content.Context
import android.content.SharedPreferences
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import androidx.biometric.BiometricPrompt
import androidx.biometric.BiometricManager
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import com.getcapacitor.CapacitorPlugin
import com.getcapacitor.JSObject
import com.getcapacitor.Plugin
import com.getcapacitor.PluginCall
import com.getcapacitor.PluginMethod
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

@CapacitorPlugin(name = "SecureVault")
class SecureVaultPlugin : Plugin() {

    private val alias = "secure_vault_key"
    private val androidKeyStore = "AndroidKeyStore"
    private val transformation = "AES/GCM/NoPadding"
    private val ivLength = 12
    private val tagLength = 128

    private fun getPrefs(): SharedPreferences {
        return context.getSharedPreferences("SecureVaultPrefs", Context.MODE_PRIVATE)
    }

    @PluginMethod
    fun isAvailable(call: PluginCall) {
        val can = BiometricManager.from(context)
            .canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG)
        val ret = JSObject()
        ret.put("value", can == BiometricManager.BIOMETRIC_SUCCESS)
        call.resolve(ret)
    }

    @PluginMethod
    fun storeKey(call: PluginCall) {
        val key = call.getString("key") ?: return call.reject("Key is required")
        try {
            val cipher = Cipher.getInstance(transformation)
            cipher.init(Cipher.ENCRYPT_MODE, getOrCreateKey())
            val iv = cipher.iv
            val encrypted = cipher.doFinal(key.toByteArray(Charsets.UTF_8))
            val combined = iv + encrypted
            val encoded = Base64.encodeToString(combined, Base64.DEFAULT)
            getPrefs().edit().putString("master_key", encoded).apply()
            call.resolve()
        } catch (e: Exception) {
            call.reject("Failed to store key: ${e.message}")
        }
    }

    @PluginMethod
    fun retrieveKey(call: PluginCall) {
        val encoded = getPrefs().getString("master_key", null)
            ?: return call.reject("No key stored")
        try {
            val combined = Base64.decode(encoded, Base64.DEFAULT)
            val iv = combined.copyOfRange(0, ivLength)
            val encrypted = combined.copyOfRange(ivLength, combined.size)

            val cipher = Cipher.getInstance(transformation)
            cipher.init(Cipher.DECRYPT_MODE, getKey(), GCMParameterSpec(tagLength, iv))

            val executor = ContextCompat.getMainExecutor(context)
            val prompt = BiometricPrompt(
                activity as FragmentActivity,
                executor,
                object : BiometricPrompt.AuthenticationCallback() {
                    override fun onAuthenticationSucceeded(result: AuthenticationResult) {
                        try {
                            val authCipher = result.cryptoObject?.cipher
                            if (authCipher == null) {
                                call.reject("Authentication succeeded but cipher unavailable")
                                return
                            }
                            val decrypted = authCipher.doFinal(encrypted)
                            val key = String(decrypted, Charsets.UTF_8)
                            val ret = JSObject()
                            ret.put("key", key)
                            call.resolve(ret)
                        } catch (e: Exception) {
                            call.reject("Decryption failed: ${e.message}")
                        }
                    }

                    override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                        call.reject("Authentication error: $errString")
                    }

                    override fun onAuthenticationFailed() {
                        call.reject("Authentication failed")
                    }
                }
            )

            val info = BiometricPrompt.PromptInfo.Builder()
                .setTitle("Unlock Gradience Wallet")
                .setSubtitle("Use your biometric credential")
                .setNegativeButtonText("Cancel")
                .setAllowedAuthenticators(BiometricManager.Authenticators.BIOMETRIC_STRONG)
                .build()

            prompt.authenticate(info, BiometricPrompt.CryptoObject(cipher))
        } catch (e: Exception) {
            call.reject("Failed to retrieve key: ${e.message}")
        }
    }

    @PluginMethod
    fun deleteKey(call: PluginCall) {
        val deleted = getPrefs().edit().remove("master_key").commit()
        try {
            val keyStore = KeyStore.getInstance(androidKeyStore)
            keyStore.load(null)
            keyStore.deleteEntry(alias)
        } catch (_: Exception) {
        }
        val ret = JSObject()
        ret.put("deleted", deleted)
        call.resolve(ret)
    }

    private fun getOrCreateKey(): SecretKey {
        return getKey() ?: generateKey()
    }

    private fun getKey(): SecretKey? {
        val keyStore = KeyStore.getInstance(androidKeyStore)
        keyStore.load(null)
        val entry = keyStore.getEntry(alias, null) as? KeyStore.SecretKeyEntry
        return entry?.secretKey
    }

    private fun generateKey(): SecretKey {
        val keyGenerator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES,
            androidKeyStore
        )
        val builder = KeyGenParameterSpec.Builder(
            alias,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setUserAuthenticationRequired(true)
            .setInvalidatedByBiometricEnrollment(true)
            .setRandomizedEncryptionRequired(true)

        keyGenerator.init(builder.build())
        return keyGenerator.generateKey()
    }
}
