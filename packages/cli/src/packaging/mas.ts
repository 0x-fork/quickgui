/**
 * Mac App Store packaging inputs.
 *
 * A MAS submission differs from a Developer ID build in four ways: a different signing identity,
 * an embedded provisioning profile, mandatory App Sandbox entitlements, and a `.pkg` produced by
 * `productbuild` rather than a disk image. Every argument list is built here so it can be asserted
 * without Xcode installed.
 */

import { CliError } from "../error.ts";
import type { MacAppStoreConfig } from "../config.ts";

/** Prefix Apple requires for the Mac App Store application signing identity. */
export const MAS_APPLICATION_IDENTITY_PREFIX = "3rd Party Mac Developer Application";
/** Prefix Apple requires for the Mac App Store installer signing identity. */
export const MAS_INSTALLER_IDENTITY_PREFIX = "3rd Party Mac Developer Installer";

/** Minimal App Sandbox entitlements QuickGUI writes when the configuration supplies none. */
export function masEntitlementsTemplate(teamIdentifier: string | undefined, identifier: string): string {
  const applicationGroup = teamIdentifier ? `${teamIdentifier}.${identifier}` : undefined;
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.app-sandbox</key>
  <true/>
  <key>com.apple.security.files.user-selected.read-write</key>
  <true/>
  <key>com.apple.security.network.client</key>
  <true/>${
    applicationGroup
      ? `\n  <key>com.apple.security.application-groups</key>\n  <array>\n    <string>${applicationGroup}</string>\n  </array>`
      : ""
  }
</dict>
</plist>
`;
}

export function validateMasConfig(config: MacAppStoreConfig): void {
  if (!config.applicationIdentity.startsWith(MAS_APPLICATION_IDENTITY_PREFIX)) {
    throw new CliError(
      `\`macos.appStore.applicationIdentity\` must start with "${MAS_APPLICATION_IDENTITY_PREFIX}"`,
    );
  }
  if (!config.installerIdentity.startsWith(MAS_INSTALLER_IDENTITY_PREFIX)) {
    throw new CliError(
      `\`macos.appStore.installerIdentity\` must start with "${MAS_INSTALLER_IDENTITY_PREFIX}"`,
    );
  }
  if (!config.provisioningProfile.endsWith(".provisionprofile")) {
    throw new CliError(
      "`macos.appStore.provisioningProfile` must point to a `.provisionprofile` file",
    );
  }
}

/** `codesign` command line for a Mac App Store application bundle. */
export function masCodesignArguments(
  appPath: string,
  identity: string,
  entitlements: string,
): string[] {
  return [
    "codesign",
    "--force",
    "--deep",
    "--timestamp",
    "--options",
    "runtime",
    "--sign",
    identity,
    "--entitlements",
    entitlements,
    appPath,
  ];
}

/** `productbuild` command line producing the submittable `.pkg`. */
export function productBuildArguments(
  appPath: string,
  installerIdentity: string,
  packagePath: string,
): string[] {
  return [
    "productbuild",
    "--component",
    appPath,
    "/Applications",
    "--sign",
    installerIdentity,
    packagePath,
  ];
}

export function masPackageFilename(name: string, version: string): string {
  const filename = `${name} ${version}.pkg`;
  if (filename.includes("/") || filename.includes("\0")) {
    throw new CliError("Application name and version cannot contain path separators on macOS");
  }
  return filename;
}
