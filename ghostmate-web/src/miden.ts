import {
  AllowedPrivateData,
  PrivateDataPermission,
  WalletAdapterNetwork,
} from "@miden-sdk/miden-wallet-adapter-base";

// Testnet — matches the rpcUrl / prover in config.ts.
export const GHOSTMATE_MIDEN_NETWORK = WalletAdapterNetwork.Testnet;

// Auto: the wallet decides which private data to share with the dApp.
export const GHOSTMATE_PRIVATE_DATA_PERMISSION = PrivateDataPermission.Auto;

// Allow the wallet to share all private data (notes, account details).
export const GHOSTMATE_ALLOWED_PRIVATE_DATA = AllowedPrivateData.All;
