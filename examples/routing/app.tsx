import { app, Window } from "@quickgui/native";
import { Button, Text, View, createRenderer, type Accessor, type NativeNode, type Style } from "@quickgui/ui";
import {
  Link,
  Outlet,
  Router,
  route,
  useLocation,
  useNavigate,
  useParams,
  useRouter,
  useSearchParams,
  type RouteDeclaration,
} from "@quickgui/ui/router";

const colors = {
  background: "#0b1020",
  panel: "#11182b",
  panelRaised: "#18223a",
  border: "#26344f",
  text: "#e8edf7",
  muted: "#8e9bb4",
  blue: "#6ea8fe",
  blueSurface: "#1d3b68",
};

const linkStyle: Style = {
  display: "flex",
  height: 34,
  alignItems: "center",
  paddingLeft: 12,
  paddingRight: 12,
  borderRadius: 8,
  color: colors.muted,
  cursor: "default",
  userSelect: "none",
};

const activeLinkStyle: Style = {
  backgroundColor: colors.blueSurface,
  color: "#dceaff",
};

function Shell() {
  const router = useRouter();
  const location = useLocation();

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        backgroundColor: colors.background,
        color: colors.text,
      }}
    >
      <View
        style={{
          display: "flex",
          height: 54,
          flexShrink: 0,
          alignItems: "center",
          paddingLeft: 78,
          paddingRight: 16,
          gap: 8,
          borderBottomWidth: 1,
          borderColor: colors.border,
        }}
      >
        <View
          style={{
            display: "flex",
            height: "100%",
            alignItems: "center",
            paddingRight: 10,
            appRegion: "drag",
          }}
        >
          <Text style={{ fontWeight: 700 }}>Router</Text>
        </View>
        <Link href="/" end style={linkStyle} activeStyle={activeLinkStyle}>
          Home
        </Link>
        <Link href="/projects" style={linkStyle} activeStyle={activeLinkStyle}>
          Projects
        </Link>
        <Link href="/settings" style={linkStyle} activeStyle={activeLinkStyle}>
          Settings
        </Link>
        <View style={{ flex: 1, height: "100%", appRegion: "drag" }} />
        <Button
          aria-label="Go back"
          disabled={!router.state().canGoBack}
          onClick={() => router.back()}
          style={{
            display: "flex",
            width: 34,
            height: 30,
            alignItems: "center",
            justifyContent: "center",
            borderRadius: 7,
            backgroundColor: colors.panelRaised,
            color: colors.text,
            cursor: "default",
            appRegion: "no-drag",
          }}
        >
          ←
        </Button>
        <Button
          aria-label="Go forward"
          disabled={!router.state().canGoForward}
          onClick={() => router.forward()}
          style={{
            display: "flex",
            width: 34,
            height: 30,
            alignItems: "center",
            justifyContent: "center",
            borderRadius: 7,
            backgroundColor: colors.panelRaised,
            color: colors.text,
            cursor: "default",
            appRegion: "no-drag",
          }}
        >
          →
        </Button>
      </View>

      <View
        style={{
          display: "flex",
          height: 34,
          flexShrink: 0,
          alignItems: "center",
          paddingLeft: 20,
          paddingRight: 20,
          backgroundColor: "#0e1526",
          borderBottomWidth: 1,
          borderColor: colors.border,
        }}
      >
        <Text style={{ fontFamily: "monospace", fontSize: 12, color: colors.muted }}>
          {location().href}
        </Text>
      </View>

      <View style={{ flex: 1, minHeight: 0 }}>
        <Outlet />
      </View>
    </View>
  );
}

function Page(props: { title: Accessor<string>; description: Accessor<string>; children?: NativeNode }) {
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        padding: 28,
        gap: 16,
        overflowY: "auto",
      }}
    >
      <Text style={{ fontSize: 28, lineHeight: 36, fontWeight: 750 }}>{props.title()}</Text>
      <Text style={{ maxWidth: 620, color: colors.muted, lineHeight: 21 }}>
        {props.description()}
      </Text>
      {props.children}
    </View>
  );
}

function Card(props: { title: string; detail: string; href: string }) {
  return (
    <Link
      href={props.href}
      style={{
        display: "flex",
        flexDirection: "column",
        width: 260,
        minHeight: 112,
        padding: 18,
        gap: 8,
        backgroundColor: colors.panel,
        borderWidth: 1,
        borderColor: colors.border,
        borderRadius: 12,
        color: colors.text,
        cursor: "default",
      }}
      activeStyle={{ borderColor: colors.blue }}
    >
      <Text style={{ fontWeight: 700 }}>{props.title}</Text>
      <Text style={{ color: colors.muted, lineHeight: 19 }}>{props.detail}</Text>
    </Link>
  );
}

function Home() {
  return (
    <Page
      title="Core-owned routing"
      description="The Rust core owns matching, decoded parameters, query parsing, active paths, and a bounded memory history. The application only renders the returned route chain."
    >
      <View style={{ display: "flex", flexWrap: "wrap", gap: 12 }}>
        <Card
          title="Dynamic parameters"
          detail="Open /projects/quickgui and read :projectId from the matched core route."
          href="/projects/quickgui?tab=overview"
        />
        <Card
          title="Nested layouts"
          detail="Settings keeps its local navigation mounted while its Outlet changes."
          href="/settings"
        />
        <Card
          title="Fallback routes"
          detail="A final wildcard catches destinations that no specific pattern matched."
          href="/this-route-does-not-exist"
        />
      </View>
    </Page>
  );
}

function Projects() {
  return (
    <Page
      title="Projects"
      description="These links push memory-history entries. Use the title-bar arrows to traverse them."
    >
      <View style={{ display: "flex", gap: 12 }}>
        <Card
          title="QuickGUI"
          detail="A native retained UI framework."
          href="/projects/quickgui?tab=overview"
        />
        <Card
          title="Screenflare"
          detail="A polished native screen recorder."
          href="/projects/screenflare?tab=activity"
        />
      </View>
    </Page>
  );
}

function Project() {
  const params = useParams();
  const search = useSearchParams();
  const navigate = useNavigate();

  return (
    <Page
      title={`Project: ${params().get("projectId") ?? ""}`}
      description="The page stays mounted when only the query changes; its reactive core snapshot updates in place."
    >
      <View
        style={{
          display: "flex",
          flexDirection: "column",
          width: 440,
          padding: 18,
          gap: 12,
          backgroundColor: colors.panel,
          borderWidth: 1,
          borderColor: colors.border,
          borderRadius: 12,
        }}
      >
        <Text style={{ color: colors.muted }}>Decoded :projectId</Text>
        <Text style={{ fontFamily: "monospace", color: colors.blue }}>
          {params().get("projectId") ?? ""}
        </Text>
        <Text style={{ marginTop: 8, color: colors.muted }}>Decoded ?tab</Text>
        <Text style={{ fontFamily: "monospace", color: colors.blue }}>
          {search().get("tab") ?? "overview"}
        </Text>
        <View style={{ display: "flex", gap: 8, marginTop: 8 }}>
          <Button
            onClick={() => navigate("?tab=overview")}
            style={{
              display: "flex",
              height: 34,
              alignItems: "center",
              justifyContent: "center",
              paddingLeft: 12,
              paddingRight: 12,
              backgroundColor: colors.panelRaised,
              color: colors.text,
              borderRadius: 8,
              cursor: "default",
            }}
          >
            Overview
          </Button>
          <Button
            onClick={() => navigate("?tab=activity")}
            style={{
              display: "flex",
              height: 34,
              alignItems: "center",
              justifyContent: "center",
              paddingLeft: 12,
              paddingRight: 12,
              backgroundColor: colors.panelRaised,
              color: colors.text,
              borderRadius: 8,
              cursor: "default",
            }}
          >
            Activity
          </Button>
          <Button
            onClick={() => navigate("?tab=activity", { replace: true })}
            style={{
              display: "flex",
              height: 34,
              alignItems: "center",
              justifyContent: "center",
              paddingLeft: 12,
              paddingRight: 12,
              backgroundColor: "#322847",
              color: colors.text,
              borderRadius: 8,
              cursor: "default",
            }}
          >
            Replace
          </Button>
        </View>
      </View>
    </Page>
  );
}

function SettingsLayout() {
  return (
    <View style={{ display: "flex", width: "100%", height: "100%" }}>
      <View
        style={{
          display: "flex",
          flexDirection: "column",
          width: 190,
          flexShrink: 0,
          padding: 16,
          gap: 8,
          backgroundColor: colors.panel,
          borderRightWidth: 1,
          borderColor: colors.border,
        }}
      >
        <Text style={{ marginBottom: 6, fontWeight: 700 }}>Settings</Text>
        <Link href="/settings" end style={linkStyle} activeStyle={activeLinkStyle}>
          General
        </Link>
        <Link href="/settings/appearance" style={linkStyle} activeStyle={activeLinkStyle}>
          Appearance
        </Link>
      </View>
      <View style={{ flex: 1, minWidth: 0 }}>
        <Outlet />
      </View>
    </View>
  );
}

function GeneralSettings() {
  return (
    <Page title="General" description="This is the index child at the same /settings path." />
  );
}

function AppearanceSettings() {
  return (
    <Page
      title="Appearance"
      description="The settings layout is shared; only this nested Outlet branch changes."
    />
  );
}

function NotFound() {
  const params = useParams();
  const navigate = useNavigate();
  return (
    <Page
      title="Route not found"
      description={`The core wildcard captured: ${params().get("*") ?? ""}`}
    >
      <Button
        onClick={() => navigate("/", { replace: true })}
        style={{
          width: 140,
          height: 38,
          backgroundColor: colors.blueSurface,
          color: colors.text,
          borderRadius: 8,
          cursor: "default",
        }}
      >
        Back home
      </Button>
    </Page>
  );
}

const routes: RouteDeclaration[] = [
  route("/", Shell, [
    route("/", Home),
    route("/projects", Projects),
    route("/projects/:projectId", Project),
    route("/settings", SettingsLayout, [route("", GeneralSettings), route("appearance", AppearanceSettings)]),
    route("*", NotFound),
  ]),
];

function ApplicationRoutes() {
  return <Router initialPath="/" routes={routes} />;
}

await app.whenReady();

function openMainWindow() {
  new Window({
    title: "QuickGUI Routing",
    width: 820,
    height: 560,
    minimumWidth: 680,
    minimumHeight: 440,
    background: colors.background,
    titleBarStyle: "hiddenInset",
    trafficLightPosition: { x: 16, y: 18 },
    renderer: createRenderer(() => <ApplicationRoutes />),
  });
}

app.onReopen((event) => {
  if (!event.hasVisibleWindows) openMainWindow();
});
openMainWindow();
