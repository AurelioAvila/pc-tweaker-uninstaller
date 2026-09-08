import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync } from "node:child_process";

// Tauri wraps the standard minisign public key and detached signature in base64.
// Verification never accesses a private key or opens a signing session.
export function verifyUpdater(file, encodedPublicKey) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "uninstaller-verify-"));
  try {
    const signature = fs.readFileSync(file + ".sig", "utf8").trim();
    for (const value of [signature, encodedPublicKey]) {
      if (!value || !/^[A-Za-z0-9+/]+={0,2}$/.test(value)) {
        throw new Error("Missing or malformed Tauri updater signature/public key.");
      }
    }
    const signaturePath = path.join(directory, "signature.minisig");
    const publicKeyPath = path.join(directory, "public.key");
    fs.writeFileSync(signaturePath, Buffer.from(signature, "base64"));
    fs.writeFileSync(publicKeyPath, Buffer.from(encodedPublicKey, "base64"));
    execFileSync(
      process.env.MINISIGN_PATH || "minisign",
      ["-V", "-m", file, "-x", signaturePath, "-p", publicKeyPath],
      { stdio: "inherit" },
    );
    return signature;
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
  }
}
