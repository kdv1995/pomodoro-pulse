#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CustomThemeColors {
    bg: String,
    fg: String,
    primary: String,
    muted_fg: String,
    accent: String,
    border: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CustomTheme {
    id: String,
    name: String,
    css_vars: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    colors: Option<CustomThemeColors>,
}

const BUILTIN_LIGHT_THEME_ID: &str = "light";
const BUILTIN_DARK_THEME_ID: &str = "dark";

fn normalize_theme_settings(settings: &mut AppSettings) {
    settings.theme = settings.theme.trim().to_string();
    settings.custom_themes = normalize_custom_themes(std::mem::take(&mut settings.custom_themes));

    if settings.theme.is_empty() {
        settings.theme = BUILTIN_LIGHT_THEME_ID.to_string();
        return;
    }

    if is_builtin_theme_id(&settings.theme) {
        return;
    }

    let has_selected_custom_theme = settings
        .custom_themes
        .iter()
        .any(|theme| theme.id == settings.theme);
    if !has_selected_custom_theme {
        settings.theme = BUILTIN_LIGHT_THEME_ID.to_string();
    }
}

fn normalize_custom_themes(custom_themes: Vec<CustomTheme>) -> Vec<CustomTheme> {
    let mut normalized = Vec::new();
    let mut seen_ids = HashSet::new();

    for custom_theme in custom_themes {
        let id = custom_theme.id.trim().to_string();
        let name = custom_theme.name.trim().to_string();
        let css_vars = custom_theme.css_vars.trim().to_string();

        if id.is_empty() || name.is_empty() || css_vars.is_empty() {
            continue;
        }
        if is_builtin_theme_id(&id) {
            continue;
        }
        if !seen_ids.insert(id.clone()) {
            continue;
        }

        normalized.push(CustomTheme {
            id,
            name,
            css_vars,
            colors: custom_theme.colors,
        });
    }

    normalized
}

fn is_builtin_theme_id(theme_id: &str) -> bool {
    matches!(theme_id, BUILTIN_LIGHT_THEME_ID | BUILTIN_DARK_THEME_ID)
}
