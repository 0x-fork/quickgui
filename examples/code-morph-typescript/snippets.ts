import { codeToKeyedTokens } from "@shikijs/magic-move/core";
import type { KeyedTokensInfo } from "@shikijs/magic-move/types";
import { createHighlighterCore } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";
import go from "shiki/langs/go.mjs";
import rust from "shiki/langs/rust.mjs";
import typescript from "shiki/langs/typescript.mjs";
import githubDark from "shiki/themes/github-dark.mjs";

export const languages = ["go", "typescript", "rust"] as const;
export type Language = (typeof languages)[number];
export const languageLabels = { go: "Go", typescript: "TypeScript", rust: "Rust" };
export const filenames = { go: "forecast.go", typescript: "forecast.ts", rust: "forecast.rs" };

// One standalone data-processing example expressed in three languages.
const forecastExample: Record<Language, string> = {
  go: `type Forecast struct {
  City      string
  Celsius   float64
}

func FormatTemperature(value float64) string {
  fahrenheit := value*9/5 + 32
  return fmt.Sprintf("%.1f°C / %.1f°F", value, fahrenheit)
}

func Warmest(forecasts []Forecast) string {
  warmest := forecasts[0]
  for _, forecast := range forecasts[1:] {
    if forecast.Celsius > warmest.Celsius {
      warmest = forecast
    }
  }
  return warmest.City + ": " + FormatTemperature(warmest.Celsius)
}`,
  typescript: `type Forecast = {
  city: string;
  celsius: number;
};

function formatTemperature(value: number): string {
  const fahrenheit = value * 9 / 5 + 32;
  return value.toFixed(1) + "°C / " + fahrenheit.toFixed(1) + "°F";
}

function warmest(forecasts: Forecast[]): string {
  const warmest = forecasts.reduce((best, forecast) =>
    forecast.celsius > best.celsius ? forecast : best
  );
  return warmest.city + ": " + formatTemperature(warmest.celsius);
}`,
  rust: `struct Forecast {
    city: String,
    celsius: f64,
}

fn format_temperature(value: f64) -> String {
    let fahrenheit = value * 9.0 / 5.0 + 32.0;
    format!("{value:.1}°C / {fahrenheit:.1}°F")
}

fn warmest(forecasts: &[Forecast]) -> String {
    let warmest = forecasts
        .iter()
        .max_by(|a, b| a.celsius.total_cmp(&b.celsius))
        .expect("at least one forecast");
    format!("{}: {}", warmest.city, format_temperature(warmest.celsius))
}`,
};

export type Snippets = Record<Language, KeyedTokensInfo>;

/** Tokenize three snippets once; release the grammar engine before opening any windows. */
export async function loadSnippets(): Promise<Snippets> {
  const highlighter = await createHighlighterCore({
    themes: [githubDark],
    langs: [go, typescript, rust],
    engine: createJavaScriptRegexEngine({ forgiving: true }),
  });
  try {
    return Object.fromEntries(
      languages.map((language) => [
        language,
        codeToKeyedTokens(highlighter, forecastExample[language], {
          lang: language,
          theme: "github-dark",
        }),
      ]),
    ) as Snippets;
  } finally {
    highlighter.dispose();
  }
}
