import { registerPlugin } from "@capacitor/core";

export interface SecureVaultPlugin {
  isAvailable(): Promise<{ value: boolean }>;
  storeKey(options: { key: string }): Promise<void>;
  retrieveKey(): Promise<{ key: string }>;
  deleteKey(): Promise<{ deleted: boolean }>;
}

export const SecureVault = registerPlugin<SecureVaultPlugin>("SecureVault");
