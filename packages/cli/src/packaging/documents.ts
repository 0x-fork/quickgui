/**
 * File-association generation for the three desktop platforms.
 *
 * Every generator here is pure: it turns validated `documentTypes` configuration into the exact
 * text each platform's installer or bundle expects, so the output can be asserted in tests without
 * running any external tool.
 */

/** Largest number of declared document types. */
export const MAX_DOCUMENT_TYPES = 64;
/** Largest number of extensions in one document type. */
export const MAX_DOCUMENT_TYPE_EXTENSIONS = 64;

export interface ResolvedDocumentType {
  name: string;
  extensions: string[];
  role: "Editor" | "Viewer";
  mimeTypes: string[];
  icon?: string;
  utTypeIdentifier?: string;
  conformsTo: string[];
  exported: boolean;
  description?: string;
}

export function xmlEscape(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

/** `CFBundleDocumentTypes` array entries for `Info.plist`. */
export function macDocumentTypesPlist(types: readonly ResolvedDocumentType[]): string {
  if (types.length === 0) return "";
  const entries = types
    .map((type) => {
      const icon = type.icon
        ? `\n      <key>CFBundleTypeIconFile</key>\n      <string>${xmlEscape(type.icon)}</string>`
        : "";
      const uti = type.utTypeIdentifier
        ? `\n      <key>LSItemContentTypes</key>\n      <array>\n        <string>${xmlEscape(
            type.utTypeIdentifier,
          )}</string>\n      </array>`
        : "";
      const mime = type.mimeTypes.length
        ? `\n      <key>CFBundleTypeMIMETypes</key>\n      <array>${type.mimeTypes
            .map((value) => `\n        <string>${xmlEscape(value)}</string>`)
            .join("")}\n      </array>`
        : "";
      return `    <dict>
      <key>CFBundleTypeName</key>
      <string>${xmlEscape(type.name)}</string>
      <key>CFBundleTypeRole</key>
      <string>${type.role}</string>
      <key>LSHandlerRank</key>
      <string>${type.exported ? "Owner" : "Alternate"}</string>
      <key>CFBundleTypeExtensions</key>
      <array>${type.extensions
        .map((extension) => `\n        <string>${xmlEscape(extension)}</string>`)
        .join("")}
      </array>${mime}${icon}${uti}
    </dict>`;
    })
    .join("\n");
  return `
  <key>CFBundleDocumentTypes</key>
  <array>
${entries}
  </array>`;
}

/** `UTExportedTypeDeclarations` / `UTImportedTypeDeclarations` arrays for `Info.plist`. */
export function macTypeDeclarationsPlist(types: readonly ResolvedDocumentType[]): string {
  const declared = types.filter((type) => type.utTypeIdentifier !== undefined);
  if (declared.length === 0) return "";
  const render = (subset: readonly ResolvedDocumentType[], key: string): string => {
    if (subset.length === 0) return "";
    const entries = subset
      .map((type) => {
        const mime = type.mimeTypes.length
          ? `\n        <key>public.mime-type</key>\n        <array>${type.mimeTypes
              .map((value) => `\n          <string>${xmlEscape(value)}</string>`)
              .join("")}\n        </array>`
          : "";
        return `    <dict>
      <key>UTTypeIdentifier</key>
      <string>${xmlEscape(type.utTypeIdentifier!)}</string>
      <key>UTTypeDescription</key>
      <string>${xmlEscape(type.description ?? type.name)}</string>
      <key>UTTypeConformsTo</key>
      <array>${type.conformsTo
        .map((value) => `\n        <string>${xmlEscape(value)}</string>`)
        .join("")}
      </array>
      <key>UTTypeTagSpecification</key>
      <dict>
        <key>public.filename-extension</key>
        <array>${type.extensions
          .map((extension) => `\n          <string>${xmlEscape(extension)}</string>`)
          .join("")}
        </array>${mime}
      </dict>
    </dict>`;
      })
      .join("\n");
    return `
  <key>${key}</key>
  <array>
${entries}
  </array>`;
  };
  return (
    render(
      declared.filter((type) => type.exported),
      "UTExportedTypeDeclarations",
    ) +
    render(
      declared.filter((type) => !type.exported),
      "UTImportedTypeDeclarations",
    )
  );
}

/** Every MIME type the Linux desktop entry advertises, deduplicated and ordered. */
export function linuxMimeTypes(types: readonly ResolvedDocumentType[]): string[] {
  const mimeTypes: string[] = [];
  for (const type of types) {
    for (const mimeType of type.mimeTypes) {
      if (!mimeTypes.includes(mimeType)) mimeTypes.push(mimeType);
    }
  }
  return mimeTypes;
}

/** A `shared-mime-info` package describing every declared type that has a MIME type. */
export function sharedMimeInfoXml(types: readonly ResolvedDocumentType[]): string {
  const entries = types
    .flatMap((type) =>
      type.mimeTypes.map(
        (mimeType) => `  <mime-type type="${xmlEscape(mimeType)}">
    <comment>${xmlEscape(type.description ?? type.name)}</comment>${type.extensions
      .map(
        (extension) =>
          `\n    <glob pattern="*.${xmlEscape(extension)}"/>`,
      )
      .join("")}
  </mime-type>`,
      ),
    )
    .join("\n");
  return `<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
${entries}
</mime-info>
`;
}

/** NSIS registry commands that register every declared extension with the installed executable. */
export function nsisFileAssociationCommands(
  identifier: string,
  executableName: string,
  types: readonly ResolvedDocumentType[],
): string[] {
  const commands: string[] = [];
  for (const type of types) {
    for (const extension of type.extensions) {
      const progId = `${identifier}.${extension}`;
      commands.push(
        `WriteRegStr SHCTX "Software\\Classes\\.${extension}" "" "${progId}"`,
        `WriteRegStr SHCTX "Software\\Classes\\${progId}" "" "${nsisString(type.name)}"`,
        `WriteRegStr SHCTX "Software\\Classes\\${progId}\\DefaultIcon" "" "$INSTDIR\\${executableName},0"`,
        `WriteRegStr SHCTX "Software\\Classes\\${progId}\\shell\\open\\command" "" '"$INSTDIR\\${executableName}" "%1"'`,
      );
    }
  }
  return commands;
}

/** NSIS registry commands that remove the associations written by {@link nsisFileAssociationCommands}. */
export function nsisFileAssociationRemovalCommands(
  identifier: string,
  types: readonly ResolvedDocumentType[],
): string[] {
  const commands: string[] = [];
  for (const type of types) {
    for (const extension of type.extensions) {
      commands.push(
        `DeleteRegKey SHCTX "Software\\Classes\\${identifier}.${extension}"`,
        `DeleteRegValue SHCTX "Software\\Classes\\.${extension}" ""`,
      );
    }
  }
  return commands;
}

/** NSIS registry commands registering each custom URL scheme with the installed executable. */
export function nsisProtocolCommands(
  executableName: string,
  protocols: readonly string[],
): string[] {
  const commands: string[] = [];
  for (const protocol of protocols) {
    commands.push(
      `WriteRegStr SHCTX "Software\\Classes\\${protocol}" "" "URL:${nsisString(protocol)}"`,
      `WriteRegStr SHCTX "Software\\Classes\\${protocol}" "URL Protocol" ""`,
      `WriteRegStr SHCTX "Software\\Classes\\${protocol}\\DefaultIcon" "" "$INSTDIR\\${executableName},0"`,
      `WriteRegStr SHCTX "Software\\Classes\\${protocol}\\shell\\open\\command" "" '"$INSTDIR\\${executableName}" "%1"'`,
    );
  }
  return commands;
}

export function nsisString(value: string): string {
  return value.replaceAll("$", "$$$$").replaceAll('"', '$\\"').replaceAll("\r", "").replaceAll("\n", " ");
}
