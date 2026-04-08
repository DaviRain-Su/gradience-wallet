#import <Capacitor/Capacitor.h>

CAP_PLUGIN(SecureVaultPlugin, "SecureVault",
    CAP_PLUGIN_METHOD(isAvailable, CAPPluginReturnPromise);
    CAP_PLUGIN_METHOD(storeKey, CAPPluginReturnPromise);
    CAP_PLUGIN_METHOD(retrieveKey, CAPPluginReturnPromise);
    CAP_PLUGIN_METHOD(deleteKey, CAPPluginReturnPromise);
)
