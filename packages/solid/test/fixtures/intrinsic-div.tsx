// Intrinsic native elements require no component or JSX runtime imports.
export function IntrinsicDiv() {
  return (
    <div style={{ fontSize: 18 }}>
      some <span style={{ textColor: "#123456" }}>text</span>
    </div>
  );
}
