// Allow clippy warnings for Leptos empty view patterns (view! {}.into_any())
// These are standard Leptos idioms for returning nothing from a component
#![allow(clippy::unit_arg)]
#![allow(clippy::unused_unit)]

use leptos::prelude::*;

// =============================================================================
// Theme & Style Constants
// =============================================================================

mod theme {
    /// Colors used throughout the application
    pub mod colors {
        pub const EMERALD: &str = "#10b981";
        pub const BLUE: &str = "#3b82f6";
        pub const GRAY_LIGHT: &str = "#e5e7eb";
        pub const GRAY_MEDIUM: &str = "#d1d5db";
        pub const GRAY_TEXT: &str = "#9ca3af";
    }

    /// CSS classes for contribution types
    pub mod classes {
        pub const CONVERSION_BG: &str = "bg-emerald-500";
        pub const CONVERSION_TEXT: &str = "text-emerald-600";
        pub const NEW_FEEDSTOCK_BG: &str = "bg-blue-500";
        pub const NEW_FEEDSTOCK_TEXT: &str = "text-blue-600";
    }
}

// =============================================================================
// Data Types
// =============================================================================

/// Type of contribution (conversion or new feedstock)
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ContributionType {
    Conversion,
    NewFeedstock,
}

impl ContributionType {
    /// Parse from string (e.g., from TOML)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "conversion" => Some(Self::Conversion),
            "new_feedstock" => Some(Self::NewFeedstock),
            _ => None,
        }
    }

    /// Get the background color class for this type
    pub fn bg_class(&self) -> &'static str {
        match self {
            Self::Conversion => theme::classes::CONVERSION_BG,
            Self::NewFeedstock => theme::classes::NEW_FEEDSTOCK_BG,
        }
    }

    /// Get the text color class for this type
    pub fn text_class(&self) -> &'static str {
        match self {
            Self::Conversion => theme::classes::CONVERSION_TEXT,
            Self::NewFeedstock => theme::classes::NEW_FEEDSTOCK_TEXT,
        }
    }

    /// Get the shape class (circle for conversion, square for new)
    pub fn shape_class(&self) -> &'static str {
        match self {
            Self::Conversion => "rounded-full", // Circle
            Self::NewFeedstock => "",           // Square (no rounding)
        }
    }

    /// Get the SVG fill color
    pub fn svg_color(&self) -> &'static str {
        match self {
            Self::Conversion => theme::colors::EMERALD,
            Self::NewFeedstock => theme::colors::BLUE,
        }
    }

    /// Short label for display
    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Conversion => "conv",
            Self::NewFeedstock => "new",
        }
    }
}

/// Top package info for a contributor
#[derive(Clone)]
struct TopPackage {
    name: String,
    downloads: u64,
}

impl TopPackage {
    fn from_toml(table: &toml::Table) -> Option<Self> {
        Some(Self {
            name: table.get("name")?.as_str()?.to_string(),
            downloads: table.get("downloads")?.as_integer()? as u64,
        })
    }
}

/// A single feedstock contribution
#[derive(Clone)]
struct FeedstockContribution {
    name: String,
    contribution_type: ContributionType,
    downloads: u64,
    #[allow(dead_code)]
    date: String,
}

impl FeedstockContribution {
    fn from_toml(table: &toml::Table) -> Option<Self> {
        Some(Self {
            name: table.get("name")?.as_str()?.to_string(),
            contribution_type: ContributionType::from_str(
                table.get("contribution_type")?.as_str()?,
            )?,
            downloads: table
                .get("downloads")
                .and_then(|v| v.as_integer())
                .unwrap_or(0) as u64,
            date: table
                .get("date")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }
}

/// Weekly activity entry: (conversions, new_feedstocks)
type WeeklyActivity = Vec<(u32, u32)>;

/// Enriched contributor statistics
#[derive(Clone)]
struct ContributorStats {
    name: String,
    conversions: u32,
    new_feedstocks: u32,
    total_downloads: u64,
    first_contribution: Option<String>,
    last_contribution: Option<String>,
    top_package: Option<TopPackage>,
    feedstocks: Vec<FeedstockContribution>,
    weekly_activity: WeeklyActivity,
}

impl ContributorStats {
    fn from_toml(table: &toml::Table) -> Option<Self> {
        Some(Self {
            name: table.get("name")?.as_str()?.to_string(),
            conversions: table.get("conversions")?.as_integer()? as u32,
            new_feedstocks: table.get("new_feedstocks")?.as_integer()? as u32,
            total_downloads: table
                .get("total_downloads")
                .and_then(|v| v.as_integer())
                .unwrap_or(0) as u64,
            first_contribution: table
                .get("first_contribution")
                .and_then(|v| v.as_str())
                .map(String::from),
            last_contribution: table
                .get("last_contribution")
                .and_then(|v| v.as_str())
                .map(String::from),
            top_package: table
                .get("top_package")
                .and_then(|v| v.as_table())
                .and_then(TopPackage::from_toml),
            feedstocks: table
                .get("feedstocks")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|f| f.as_table().and_then(FeedstockContribution::from_toml))
                        .collect()
                })
                .unwrap_or_default(),
            weekly_activity: table
                .get("weekly_activity")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|week| {
                            let week_arr = week.as_array()?;
                            let conv = week_arr.first()?.as_integer()? as u32;
                            let new = week_arr.get(1)?.as_integer()? as u32;
                            Some((conv, new))
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    /// Total contributions (conversions + new feedstocks)
    #[allow(dead_code)]
    fn total(&self) -> u32 {
        self.conversions + self.new_feedstocks
    }

    /// Average downloads per package
    #[allow(dead_code)]
    fn avg_downloads(&self) -> u64 {
        let total = self.total();
        if total > 0 {
            self.total_downloads / total as u64
        } else {
            0
        }
    }
}

// =============================================================================
// Reusable UI Components
// =============================================================================

/// A small shape indicator (circle or square) for contribution type
#[component]
fn ShapeIndicator(
    contribution_type: ContributionType,
    #[prop(default = "w-2 h-2")] size: &'static str,
) -> impl IntoView {
    let class = format!(
        "{} {} {}",
        size,
        contribution_type.bg_class(),
        contribution_type.shape_class()
    );
    view! { <span class=class></span> }
}

/// A stat card with label and value
#[component]
fn StatCard(
    label: &'static str,
    value: String,
    #[prop(default = "text-gray-900")] value_class: &'static str,
    #[prop(optional)] subtitle: Option<&'static str>,
) -> impl IntoView {
    view! {
        <div class="bg-white rounded-md p-3 border border-gray-100">
            <div class="text-xs text-gray-500 mb-1">{label}</div>
            <div class=format!("text-xl font-bold tabular-nums {}", value_class)>{value}</div>
            {subtitle.map(|s| view! {
                <div class="text-xs text-gray-400 mt-1">{s}</div>
            })}
        </div>
    }
}

#[component]
fn App() -> impl IntoView {
    let stats = include_str!("stats.toml");
    let toml_data: toml::Table = toml::from_str(stats).unwrap();

    let converted_recipes = toml_data
        .get("recipe_v1_count")
        .unwrap()
        .as_integer()
        .unwrap() as u32;
    let total_recipes = toml_data
        .get("total_feedstocks")
        .unwrap()
        .as_integer()
        .unwrap() as u32;

    let mut recently_updated = toml_data
        .get("recently_updated")
        .and_then(|v| v.as_table())
        .map(|table| {
            table
                .iter()
                .map(|(name, date)| (name.clone(), date.as_str().unwrap_or("").to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    recently_updated.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by date descending

    let last_updated = toml_data
        .get("last_updated")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let top_unconverted = toml_data
        .get("top_unconverted_by_downloads")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let table = item.as_table()?;
                    let name = table.get("name")?.as_str()?.to_string();
                    let downloads = table.get("downloads")?.as_integer()?;
                    let recipe_type = table.get("recipe_type")?.as_str()?.to_string();
                    Some((name, downloads as u64, recipe_type))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Extract top contributors for leaderboard with enriched data
    let top_contributors: Vec<ContributorStats> = toml_data
        .get("top_contributors")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_table().and_then(ContributorStats::from_toml))
                .collect()
        })
        .unwrap_or_default();

    view! {
        <div class="min-h-screen bg-gray-50">
            <header class="text-center py-10 px-4">
                <h1 class="text-5xl md:text-6xl font-bold text-gray-900 mb-4 tracking-tight">
                    "Are we recipe v1 yet?"
                </h1>
                <p class="text-base text-gray-500 max-w-2xl mx-auto mb-6">
                    "Tracking conda-forge's migration from meta.yaml to recipe.yaml"
                </p>
                <InfoAccordion />
            </header>
            <div class="max-w-6xl mx-auto px-4 pb-8">
                <main class="bg-white rounded-lg p-8 shadow-sm border border-gray-200 hover:shadow-md transition-shadow duration-200">
                    <div class="grid md:grid-cols-2 gap-12 items-center">
                        <MigrationChart converted=converted_recipes total=total_recipes />
                        <MigrationStats converted=converted_recipes total=total_recipes />
                    </div>
                </main>
                <div class="mt-8">
                    <RecentlyUpdated feedstocks=recently_updated last_updated=last_updated.to_string() />
                </div>
                <div class="mt-8">
                    <Leaderboard contributors=top_contributors />
                </div>
                <div class="mt-8">
                    <TopUnconvertedRanking feedstocks=top_unconverted />
                </div>
            </div>
            <div class="max-w-6xl mx-auto px-4 mt-8 mb-8">
                <a href="https://rattler.build" target="_blank" class="block rounded-lg ring-0 ring-gray-900 hover:ring-2 transition-all duration-150">
                    <img
                        src="./banner.png"
                        alt="rattler-build: A fast package build tool for Conda packages written in Rust"
                        class="w-full rounded-lg shadow-lg hover:shadow-xl transition-shadow duration-150"
                    />
                </a>
            </div>
        </div>
    }
}

#[component]
fn InfoAccordion() -> impl IntoView {
    let (expanded, set_expanded) = signal(false);

    view! {
        <div class="max-w-6xl mx-auto">
            <button
                on:click=move |_| set_expanded.update(|v| *v = !*v)
                class="inline-flex items-center gap-2 py-2 px-4 text-gray-500 hover:text-gray-700 border border-gray-300 hover:border-gray-400 rounded-full transition-all duration-150 text-sm font-medium"
            >
                <span>"Learn more"</span>
                <svg
                    class=move || format!(
                        "w-4 h-4 transition-transform duration-200 {}",
                        if expanded.get() { "rotate-180" } else { "" }
                    )
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                >
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                </svg>
            </button>
            <div class=move || format!(
                "accordion-content {}",
                if expanded.get() { "expanded" } else { "" }
            )>
                <div>
                    <div class="grid grid-cols-1 md:grid-cols-3 gap-6 pt-4 pb-2">
                        <div class="bg-white rounded-lg p-6 shadow-sm border border-gray-200 hover:shadow-md hover:border-gray-300 transition-all duration-200">
                            <h3 class="text-lg font-semibold text-gray-900 mb-3 tracking-tight">"What is " <strong>"conda-forge"</strong> "?"</h3>
                            <p class="text-gray-600 mb-3 leading-relaxed text-sm">
                                <strong class="text-gray-700">"conda-forge"</strong> " is a community-driven collection of " <strong class="text-gray-700">"conda packages"</strong> ". It's an open-source project that provides high-quality, "
                                "up-to-date conda packages for scientific computing and data science ecosystems."
                            </p>
                            <p class="text-gray-600 mb-3 leading-relaxed text-sm">
                                "With over " <strong class="text-gray-700">"26,000 feedstocks"</strong> ", conda-forge makes it easy to install software packages using " <strong class="text-gray-700">"conda"</strong> "."
                            </p>
                            <p class="text-gray-600 text-sm">
                                "Visit "
                                <a href="https://conda-forge.org" class="text-blue-600 hover:text-blue-800 underline transition-colors duration-150">"conda-forge.org"</a>
                                " or explore the "
                                <a href="https://github.com/conda-forge" class="text-blue-600 hover:text-blue-800 underline transition-colors duration-150">"GitHub organization"</a>
                                "."
                            </p>
                        </div>

                        <div class="bg-white rounded-lg p-6 shadow-sm border border-gray-200 hover:shadow-md hover:border-gray-300 transition-all duration-200">
                            <h3 class="text-lg font-semibold text-gray-900 mb-3 tracking-tight">"What is " <strong>"Recipe v1"</strong> "?"</h3>
                            <p class="text-gray-600 mb-3 leading-relaxed text-sm">
                                <strong class="text-gray-700">"Recipe v1"</strong> " is the new standardized format for " <strong class="text-gray-700">"conda package recipes"</strong> ", replacing the legacy " <strong class="text-gray-700">"meta.yaml"</strong> " format. "
                                "It provides better structure, validation, and tooling support."
                            </p>
                            <p class="text-gray-600 text-sm">
                                "Learn more in "
                                <a href="https://github.com/conda/ceps/blob/main/cep-0013.md" class="text-blue-600 hover:text-blue-800 underline transition-colors duration-150">"CEP-0013"</a>
                                " and "
                                <a href="https://github.com/conda/ceps/blob/main/cep-0014.md" class="text-blue-600 hover:text-blue-800 underline transition-colors duration-150">"CEP-0014"</a>
                                "."
                            </p>
                        </div>

                        <div class="bg-white rounded-lg p-6 shadow-sm border border-gray-200 hover:shadow-md hover:border-gray-300 transition-all duration-200">
                            <h3 class="text-lg font-semibold text-gray-900 mb-3 tracking-tight">"What is " <strong>"rattler-build"</strong> "?"</h3>
                            <p class="text-gray-600 mb-3 leading-relaxed text-sm">
                                <strong class="text-gray-700">"rattler-build"</strong> " is a fast, modern build tool for " <strong class="text-gray-700">"conda packages"</strong> " written in " <strong class="text-gray-700">"Rust"</strong> ". It's designed to work with the new " <strong class="text-gray-700">"Recipe v1"</strong> " format "
                                "and provides significant performance improvements over " <strong class="text-gray-700">"conda-build"</strong> "."
                            </p>
                            <p class="text-gray-600 text-sm">
                                "Visit "
                                <a href="https://rattler.build" class="text-blue-600 hover:text-blue-800 underline transition-colors duration-150">"rattler.build"</a>
                                " to learn more."
                            </p>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn MigrationChart(converted: u32, total: u32) -> impl IntoView {
    let percentage = converted as f64 / total as f64 * 100.0;

    // SVG circle constants
    const CIRCLE_RADIUS: f64 = 80.0;
    const DEGREES_PER_PERCENT: f64 = 3.6; // 360 degrees / 100 percent

    // Calculate circumference: 2π * radius
    let circumference = 2.0 * std::f64::consts::PI * CIRCLE_RADIUS;

    // Convert percentage to degrees, then to arc length
    let converted_angle = percentage * DEGREES_PER_PERCENT;
    let arc_length = (converted_angle / 360.0) * circumference;
    let remaining_length = circumference - arc_length;

    // CSS variables for the animation
    let style_vars = format!(
        "--progress-arc: {:.2}; --progress-remaining: {:.2};",
        arc_length, remaining_length
    );

    view! {
        <div class="flex flex-col items-center">
            <h2 class="text-2xl font-semibold text-gray-900 mb-8 tracking-tight">"Migration Progress"</h2>
            <div class="relative w-64 h-64">
                <svg class="w-full h-full transform -rotate-90" viewBox="0 0 200 200">
                    // Background circle (full circumference)
                    <circle
                        cx="100"
                        cy="100"
                        r="80"
                        fill="none"
                        stroke="#e5e7eb"
                        stroke-width="20"
                    />
                    // Progress circle (partial circumference based on percentage)
                    <circle
                        cx="100"
                        cy="100"
                        r="80"
                        fill="none"
                        stroke="#F9C500"
                        stroke-width="20"
                        stroke-linecap="round"
                        class="progress-circle"
                        style=style_vars
                    />
                </svg>
                <div class="absolute inset-0 flex items-center justify-center">
                    <div class="text-center">
                        <div class="text-3xl font-bold text-gray-900 tabular-nums">{format!("{:.1}%", percentage)}</div>
                        <div class="text-sm text-gray-500">"Complete"</div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn MigrationStats(converted: u32, total: u32) -> impl IntoView {
    view! {
        <div class="space-y-6">
            <h2 class="text-2xl font-semibold text-gray-900 tracking-tight text-center">"Migration Statistics"</h2>

            <div class="flex items-end justify-center gap-3">
                <div class="text-center">
                    <div class="text-xs font-semibold text-emerald-600 uppercase tracking-wide mb-1">"Converted"</div>
                    <div class="text-4xl font-bold text-emerald-600 tabular-nums">{converted.to_string()}</div>
                </div>
                <div class="text-4xl font-light text-gray-300 pb-1">"/"</div>
                <div class="text-center">
                    <div class="text-xs font-semibold text-gray-500 uppercase tracking-wide mb-1">"Remaining"</div>
                    <div class="text-4xl font-bold text-gray-700 tabular-nums">{(total - converted).to_string()}</div>
                </div>
            </div>

            <div class="text-center text-sm text-gray-500">
                "out of " <span class="tabular-nums font-medium">{total.to_string()}</span> " total feedstocks"
            </div>
        </div>
    }
}

#[component]
fn RecentlyUpdated(feedstocks: Vec<(String, String)>, last_updated: String) -> impl IntoView {
    if feedstocks.is_empty() {
        return view! {}.into_any();
    }

    let formatted_date = format_date(&last_updated);

    view! {
        <div class="bg-white rounded-lg p-8 shadow-sm border border-gray-200 hover:shadow-md transition-shadow duration-200">
            <div class="flex items-center justify-between mb-4">
                <h2 class="text-lg font-semibold text-gray-900 tracking-tight">"Recently Updated to Recipe v1"</h2>
                <span class="text-xs text-gray-400">"Updated " {formatted_date}</span>
            </div>
            <div class="flex items-center text-xs font-semibold text-gray-500 uppercase tracking-wide mb-3">
                <span>"Recipe Name"</span>
                <span class="flex-1"></span>
                <span>"Change Detected"</span>
            </div>
            <ul class="space-y-1">
                {feedstocks.into_iter().map(|(name, date)| {
                    let formatted_date = format_date(&date);
                    let github_url = format!("https://github.com/conda-forge/{}", name);
                    let display_name = name.replace("-feedstock", "");
                    view! {
                        <li>
                            <a
                                href=github_url
                                target="_blank"
                                rel="noopener noreferrer"
                                class="flex items-center text-gray-700 py-2 -mx-2 px-2 rounded hover:bg-gray-50 transition-colors duration-150 cursor-pointer"
                            >
                                <span class="font-medium text-blue-600">{display_name}</span>
                                <span class="flex-1 border-b border-dotted border-gray-300 mx-3"></span>
                                <span class="text-sm text-gray-500 tabular-nums">{formatted_date}</span>
                            </a>
                        </li>
                    }
                }).collect::<Vec<_>>()}
            </ul>
        </div>
    }.into_any()
}

/// Achievement definition with emoji, name, and threshold
struct Achievement {
    emoji: &'static str,
    name: &'static str,
    tooltip: &'static str,
    threshold: u32,
}

/// All achievement definitions - single source of truth
mod achievements {
    use super::Achievement;

    // Total contribution achievements
    pub const TOTAL: &[Achievement] = &[
        Achievement {
            emoji: "🦄",
            name: "Conda Mythic",
            tooltip: "Conda Mythic (500+ total v1 contributions)",
            threshold: 500,
        },
        Achievement {
            emoji: "👑",
            name: "Forge Legend",
            tooltip: "Forge Legend (200+ v1 contributions)",
            threshold: 200,
        },
        Achievement {
            emoji: "🚀",
            name: "Master Smith",
            tooltip: "Master Smith (100+ v1 contributions)",
            threshold: 100,
        },
        Achievement {
            emoji: "💫",
            name: "Forge Smith",
            tooltip: "Forge Smith (50+ v1 contributions)",
            threshold: 50,
        },
        Achievement {
            emoji: "🌟",
            name: "Recipe Crafter",
            tooltip: "Recipe Crafter (25+ v1 contributions)",
            threshold: 25,
        },
        Achievement {
            emoji: "⭐",
            name: "Forge Apprentice",
            tooltip: "Forge Apprentice (10+ v1 contributions)",
            threshold: 10,
        },
    ];

    // Conversion-specific achievements
    pub const CONVERSIONS: &[Achievement] = &[
        Achievement {
            emoji: "💎",
            name: "Transmutation Master",
            tooltip: "Transmutation Master (250+ v1 conversions)",
            threshold: 250,
        },
        Achievement {
            emoji: "🔥",
            name: "Migration Furnace",
            tooltip: "Migration Furnace (100+ v1 conversions)",
            threshold: 100,
        },
        Achievement {
            emoji: "⚡",
            name: "YAML Wizard",
            tooltip: "YAML Wizard (50+ v1 conversions)",
            threshold: 50,
        },
        Achievement {
            emoji: "🔄",
            name: "Recipe Translator",
            tooltip: "Recipe Translator (10+ v1 conversions)",
            threshold: 10,
        },
    ];

    // New feedstock achievements
    pub const NEW_FEEDSTOCKS: &[Achievement] = &[
        Achievement {
            emoji: "🏞️",
            name: "Conda Terraformer",
            tooltip: "Conda Terraformer (250+ new v1 feedstocks)",
            threshold: 250,
        },
        Achievement {
            emoji: "🌲",
            name: "Ecosystem Grower",
            tooltip: "Ecosystem Grower (100+ new v1 feedstocks)",
            threshold: 100,
        },
        Achievement {
            emoji: "🌳",
            name: "Package Cultivator",
            tooltip: "Package Cultivator (50+ new v1 feedstocks)",
            threshold: 50,
        },
        Achievement {
            emoji: "🌱",
            name: "Feedstock Farmer",
            tooltip: "Feedstock Farmer (10+ new v1 feedstocks)",
            threshold: 10,
        },
    ];
}

/// Get the highest achievement earned for a given value from a list of achievements
fn get_achievement(
    value: u32,
    achievements: &[Achievement],
) -> Option<(&'static str, &'static str)> {
    achievements
        .iter()
        .find(|a| value >= a.threshold)
        .map(|a| (a.emoji, a.tooltip))
}

/// Compute achievement badges for a contributor based on their stats
/// Returns vec of (emoji, name) tuples
fn compute_achievements(
    conversions: u32,
    new_feedstocks: u32,
) -> Vec<(&'static str, &'static str)> {
    let mut result = Vec::new();
    let total = conversions + new_feedstocks;

    if let Some(achievement) = get_achievement(total, achievements::TOTAL) {
        result.push(achievement);
    }
    if let Some(achievement) = get_achievement(conversions, achievements::CONVERSIONS) {
        result.push(achievement);
    }
    if let Some(achievement) = get_achievement(new_feedstocks, achievements::NEW_FEEDSTOCKS) {
        result.push(achievement);
    }

    result
}

/// Component for a single contributor row with expandable details
#[component]
fn ContributorRow(index: usize, contributor: ContributorStats) -> impl IntoView {
    let (expanded, set_expanded) = signal(false);

    let total = contributor.conversions + contributor.new_feedstocks;
    let github_url = format!("https://github.com/{}", contributor.name);

    // Medal emoji for top 3
    let medal = match index {
        0 => Some("🥇"),
        1 => Some("🥈"),
        2 => Some("🥉"),
        _ => None,
    };

    // Compute achievements for this contributor
    let achievements = compute_achievements(contributor.conversions, contributor.new_feedstocks);

    // Clone values for use in closures
    let name = contributor.name.clone();
    let conversions = contributor.conversions;
    let new_feedstocks = contributor.new_feedstocks;
    let total_downloads = contributor.total_downloads;
    let first_contribution = contributor.first_contribution.clone();
    let last_contribution = contributor.last_contribution.clone();
    let top_package = contributor.top_package.clone();
    let feedstocks = contributor.feedstocks.clone();
    let weekly_activity = contributor.weekly_activity.clone();

    view! {
        <li class="border-b border-dashed border-gray-200">
            <div
                on:click=move |_| set_expanded.update(|v| *v = !*v)
                class="flex items-center py-2 -mx-2 px-2 rounded hover:bg-gray-50 transition-colors duration-150 cursor-pointer"
            >
                // Expander chevron at the front
                <span class="w-6 flex items-center justify-center mr-1">
                    <svg
                        class=move || format!(
                            "w-4 h-4 text-gray-400 transition-transform duration-200 {}",
                            if expanded.get() { "rotate-180" } else { "" }
                        )
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                    >
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                    </svg>
                </span>
                {if let Some(emoji) = medal {
                    view! {
                        <span class="w-8 h-8 flex items-center justify-center text-xl">
                            {emoji}
                        </span>
                    }.into_any()
                } else {
                    view! {
                        <span class="w-8 h-8 flex items-center justify-center text-xs tabular-nums text-gray-400">
                            {format!("{}", index + 1)}
                        </span>
                    }.into_any()
                }}
                <span class="flex-1 font-medium text-blue-600">
                    <a
                        href=github_url.clone()
                        target="_blank"
                        rel="noopener noreferrer"
                        on:click=move |e| e.stop_propagation()
                        class="hover:underline"
                    >
                        {name.clone()}
                    </a>
                    {if !achievements.is_empty() {
                        view! {
                            <span class="ml-2 text-base">
                                {achievements.iter().map(|(emoji, achievement_name)| {
                                    view! {
                                        <span title=*achievement_name>{*emoji}</span>
                                    }
                                }).collect::<Vec<_>>()}
                            </span>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                </span>
                <span class="w-24 text-center text-sm text-emerald-600 tabular-nums">
                    {conversions}
                </span>
                <span class="w-24 text-center text-sm text-blue-600 tabular-nums">
                    {new_feedstocks}
                </span>
                <span class="w-16 text-right text-sm font-medium text-gray-700 tabular-nums">
                    {total}
                </span>
            </div>

            // Expanded details panel
            <div class=move || format!(
                "accordion-content {}",
                if expanded.get() { "expanded" } else { "" }
            )>
                <div class="overflow-hidden">
                    <div class="pl-14 pr-4 pb-4">
                        <ContributorDetails
                        name=name.clone()
                        total_downloads=total_downloads
                        conversions=conversions
                        new_feedstocks=new_feedstocks
                        first_contribution=first_contribution.clone()
                        last_contribution=last_contribution.clone()
                        top_package=top_package.clone()
                        feedstocks=feedstocks.clone()
                        weekly_activity=weekly_activity.clone()
                    />
                    </div>
                </div>
            </div>
        </li>
    }
}

/// Weekly activity sparkline showing stacked bars for the last 20 weeks
#[component]
fn ActivitySparkline(weekly_activity: WeeklyActivity) -> impl IntoView {
    // Find max total for scaling
    let max_total = weekly_activity
        .iter()
        .map(|(c, n)| c + n)
        .max()
        .unwrap_or(1)
        .max(1); // Ensure at least 1 to avoid division by zero

    // SVG dimensions - bigger than before
    let bar_width = 8;
    let bar_gap = 3;
    let height = 32;
    let bar_count = weekly_activity.len();
    let total_width = bar_count * (bar_width + bar_gap);

    // Generate bars with tooltip info (reversed so most recent is on right)
    let bars: Vec<_> = weekly_activity
        .iter()
        .rev()
        .enumerate()
        .map(|(i, (conv, new_fs))| {
            let total = conv + new_fs;
            let bar_height = if total > 0 {
                ((total as f64 / max_total as f64) * (height as f64 - 4.0)).max(2.0)
            } else {
                0.0
            };

            let new_height = if total > 0 {
                (*new_fs as f64 / total as f64) * bar_height
            } else {
                0.0
            };
            let conv_height = bar_height - new_height;

            let x = i * (bar_width + bar_gap);
            let conv_y = height as f64 - bar_height;
            let new_y = conv_y + conv_height;

            // Weeks ago for tooltip (i is reversed, so 0 = most recent)
            let weeks_ago = bar_count - 1 - i;
            let tooltip = if total > 0 {
                if weeks_ago == 0 {
                    format!("This week: {} conv, {} new", conv, new_fs)
                } else if weeks_ago == 1 {
                    format!("1 week ago: {} conv, {} new", conv, new_fs)
                } else {
                    format!("{} weeks ago: {} conv, {} new", weeks_ago, conv, new_fs)
                }
            } else {
                if weeks_ago == 0 {
                    "This week: no activity".to_string()
                } else if weeks_ago == 1 {
                    "1 week ago: no activity".to_string()
                } else {
                    format!("{} weeks ago: no activity", weeks_ago)
                }
            };

            (x, conv_y, conv_height, new_y, new_height, total, tooltip)
        })
        .collect();

    // Calculate a nice reference line value (round to nearest 5 or 10)
    let reference_value = if max_total >= 20 {
        ((max_total + 9) / 10) * 10 // Round up to nearest 10
    } else if max_total >= 5 {
        ((max_total + 4) / 5) * 5 // Round up to nearest 5
    } else {
        max_total
    };
    let reference_y = height as f64 - ((reference_value as f64 / max_total as f64) * (height as f64 - 4.0));

    // Calculate label width based on number of digits
    let label_width = if reference_value >= 100 { 25 } else if reference_value >= 10 { 18 } else { 12 };

    view! {
        <svg
            width=total_width + label_width + 4
            height=height
            class="inline-block align-middle overflow-visible"
            viewBox=format!("0 0 {} {}", total_width + label_width + 4, height)
        >
            // Reference line with label
            <line
                x1="0"
                y1=reference_y
                x2={total_width}
                y2=reference_y
                stroke=theme::colors::GRAY_MEDIUM
                stroke-width="1"
                stroke-dasharray="2,2"
            />
            <text
                x={total_width + 3}
                y={reference_y + 3.0}
                font-size="9"
                fill=theme::colors::GRAY_TEXT
            >
                {reference_value}
            </text>
            // Baseline
            <line
                x1="0"
                y1={height - 1}
                x2={total_width}
                y2={height - 1}
                stroke=theme::colors::GRAY_LIGHT
                stroke-width="1"
            />
            // Bars with tooltips
            {bars.into_iter().map(|(x, conv_y, conv_height, new_y, new_height, total, tooltip)| {
                view! {
                    <g>
                        {if total == 0 {
                            // Show thin gray placeholder for empty weeks
                            view! {
                                <rect
                                    x=x
                                    y={height - 2}
                                    width=bar_width
                                    height="1"
                                    fill=theme::colors::GRAY_MEDIUM
                                    rx="1"
                                >
                                    <title>{tooltip.clone()}</title>
                                </rect>
                            }.into_any()
                        } else {
                            view! {
                                <g>
                                    // Conversions (emerald) - top part
                                    {if conv_height > 0.0 {
                                        view! {
                                            <rect
                                                x=x
                                                y=conv_y
                                                width=bar_width
                                                height=conv_height
                                                fill=ContributionType::Conversion.svg_color()
                                                rx="1"
                                            >
                                                <title>{tooltip.clone()}</title>
                                            </rect>
                                        }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }}
                                    // New feedstocks (blue) - bottom part
                                    {if new_height > 0.0 {
                                        view! {
                                            <rect
                                                x=x
                                                y=new_y
                                                width=bar_width
                                                height=new_height
                                                fill=ContributionType::NewFeedstock.svg_color()
                                                rx="1"
                                            >
                                                <title>{tooltip.clone()}</title>
                                            </rect>
                                        }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }}
                                </g>
                            }.into_any()
                        }}
                    </g>
                }
            }).collect::<Vec<_>>()}
        </svg>
    }
}

/// Expanded details panel for a contributor
#[component]
fn ContributorDetails(
    #[allow(unused)] name: String,
    total_downloads: u64,
    conversions: u32,
    new_feedstocks: u32,
    first_contribution: Option<String>,
    last_contribution: Option<String>,
    top_package: Option<TopPackage>,
    feedstocks: Vec<FeedstockContribution>,
    weekly_activity: WeeklyActivity,
) -> impl IntoView {
    let total = conversions + new_feedstocks;
    let avg_downloads = if total > 0 {
        total_downloads / total as u64
    } else {
        0
    };

    view! {
        <div class="stats-card bg-gray-50 rounded-lg p-4 border border-gray-200 shadow-sm">
            // Stats cards row
            <div class="grid grid-cols-2 md:grid-cols-4 gap-3 mb-4">
                <StatCard
                    label="Conversions"
                    value=conversions.to_string()
                    value_class=ContributionType::Conversion.text_class()
                />
                <StatCard
                    label="New Feedstocks"
                    value=new_feedstocks.to_string()
                    value_class=ContributionType::NewFeedstock.text_class()
                />
                <StatCard
                    label="Total Downloads*"
                    value=format!("~{}", format_downloads(total_downloads))
                />
                <StatCard
                    label="Avg per Package*"
                    value=format!("~{}", format_downloads(avg_downloads))
                />
            </div>

            // Activity timeline and Top package row
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3 mb-4">
                // Activity timeline card
                <div class="bg-white rounded-md p-3 border border-gray-100">
                    <div class="flex items-center justify-between mb-2">
                        <div class="text-xs font-semibold text-gray-500 uppercase tracking-wide">"Activity Timeline"</div>
                        <div class="flex items-center gap-3 text-xs text-gray-400">
                            <span class="flex items-center gap-1">
                                <ShapeIndicator contribution_type=ContributionType::Conversion />
                                {ContributionType::Conversion.short_label()}
                            </span>
                            <span class="flex items-center gap-1">
                                <ShapeIndicator contribution_type=ContributionType::NewFeedstock />
                                {ContributionType::NewFeedstock.short_label()}
                            </span>
                            <span>"· 20 wks"</span>
                        </div>
                    </div>
                    <ActivitySparkline weekly_activity=weekly_activity.clone() />
                    <div class="flex justify-between text-xs text-gray-400 mt-2">
                        {if let Some(ref date) = first_contribution {
                            view! { <span>"First: "{format_date(date)}</span> }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                        {if let Some(ref date) = last_contribution {
                            view! { <span>"Latest: "{format_date(date)}</span> }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                    </div>
                </div>

                // Top package card
                {if let Some(ref pkg) = top_package {
                    let package_name = pkg.name.replace("-feedstock", "");
                    let prefix_url = format!("https://prefix.dev/channels/conda-forge/packages/{}", package_name);
                    view! {
                        <a
                            href=prefix_url
                            target="_blank"
                            rel="noopener noreferrer"
                            class="bg-white rounded-md p-3 border border-gray-100 flex flex-col justify-center hover:bg-gray-50 transition-colors duration-150"
                        >
                            <div class="text-xs text-gray-500 mb-1">"Top Package"</div>
                            <div class="text-lg font-semibold text-emerald-600 mb-1 hover:underline">{package_name}</div>
                            <div class="text-sm text-gray-500 tabular-nums">{"~"}{format_downloads(pkg.downloads)}" downloads*"</div>
                        </a>
                    }.into_any()
                } else {
                    view! {
                        <div class="bg-white rounded-md p-3 border border-gray-100 flex flex-col justify-center items-center text-gray-400">
                            <div class="text-xs">"No package data"</div>
                        </div>
                    }.into_any()
                }}
            </div>

            // Feedstocks list (top 10 by downloads)
            {if !feedstocks.is_empty() {
                view! {
                    <div class="bg-white rounded-md p-3 border border-gray-100">
                        <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wide mb-2">"Top Feedstocks by Downloads*"</h4>
                        <div class="grid grid-cols-1 md:grid-cols-2 gap-1">
                            {feedstocks.into_iter().map(|f| {
                                let github_url = format!("https://github.com/conda-forge/{}", f.name);
                                let display_name = f.name.replace("-feedstock", "");
                                let shape_class = format!(
                                    "w-2 h-2 {} {} mr-2 flex-shrink-0",
                                    f.contribution_type.bg_class(),
                                    f.contribution_type.shape_class()
                                );

                                view! {
                                    <a
                                        href=github_url
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        class="flex items-center text-sm py-1 group"
                                    >
                                        <span class=shape_class></span>
                                        <span class="font-medium text-blue-600 truncate flex-1 group-hover:underline">{display_name}</span>
                                        <span class="text-xs text-gray-500 ml-2 tabular-nums w-16 text-right">{"~"}{format_downloads(f.downloads)}</span>
                                    </a>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}

            // Footnote for download data source
            <p class="text-xs text-gray-400 mt-3">
                "* Download counts from "
                <a href="https://prefix.dev/channels/conda-forge" target="_blank" rel="noopener noreferrer" class="text-blue-500 hover:underline">"prefix.dev"</a>
                ", summed across top 10 versions per package."
            </p>
        </div>
    }
}

/// Helper function to format download counts
fn format_downloads(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

/// Helper function to format ISO date to human readable
fn format_date(iso_date: &str) -> String {
    if let Some(date_part) = iso_date.split('T').next() {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
            return date.format("%b %d, %Y").to_string();
        }
    }
    iso_date.to_string()
}

#[component]
fn Leaderboard(contributors: Vec<ContributorStats>) -> impl IntoView {
    if contributors.is_empty() {
        return view! {}.into_any();
    }

    // Calculate totals for summary
    let total_conversions: u32 = contributors.iter().map(|c| c.conversions).sum();
    let total_new_feedstocks: u32 = contributors.iter().map(|c| c.new_feedstocks).sum();

    view! {
        <div class="bg-white rounded-lg p-8 shadow-sm border border-gray-200 hover:shadow-md transition-shadow duration-200">
            <div class="mb-6">
                <h2 class="text-2xl font-semibold text-gray-900 mb-2 tracking-tight">
                    "Recipe v1 Contributors"
                </h2>
                <p class="text-gray-500 leading-relaxed mb-3">
                    "A huge thank you to everyone helping migrate conda-forge to Recipe v1! "
                    "Your contributions make the ecosystem better for everyone."
                </p>
                <div class="flex gap-6 mt-4 mb-4">
                    <div class="text-center">
                        <div class="text-2xl font-bold text-emerald-600 tabular-nums">{total_conversions}</div>
                        <div class="text-xs text-gray-500 uppercase tracking-wide">"Conversions"</div>
                    </div>
                    <div class="text-center">
                        <div class="text-2xl font-bold text-blue-600 tabular-nums">{total_new_feedstocks}</div>
                        <div class="text-xs text-gray-500 uppercase tracking-wide">"New Feedstocks"</div>
                    </div>
                </div>
                <div class="text-xs text-gray-400 pt-3 space-y-2">
                    <details class="cursor-pointer">
                        <summary class="hover:text-gray-600 transition-colors">"How do we track contributions?"</summary>
                        <div class="mt-2 space-y-1 text-gray-500">
                            <p>
                                <strong class="text-emerald-600">"Conversions"</strong>
                                " are detected when a human contributor adds a recipe.yaml to an existing feedstock. "
                                "We find the first commit that introduced recipe.yaml and credit the commit author."
                            </p>
                            <p>
                                <strong class="text-blue-600">"New Feedstocks"</strong>
                                " are detected when the recipe.yaml was added by a bot (automated staging). "
                                "In this case, we credit the recipe maintainers listed in the recipe.yaml file."
                            </p>
                        </div>
                    </details>
                    <details class="cursor-pointer">
                        <summary class="hover:text-gray-600 transition-colors">"Achievement Legend"</summary>
                        <div class="mt-3 overflow-x-auto">
                            <table class="w-full text-left text-gray-500 text-base">
                                <thead>
                                    <tr class="border-b border-gray-200">
                                        <th class="pb-2 font-medium text-gray-600">"Category"</th>
                                        <th class="pb-2 font-medium text-gray-600">"10+"</th>
                                        <th class="pb-2 font-medium text-gray-600">"25+"</th>
                                        <th class="pb-2 font-medium text-gray-600">"50+"</th>
                                        <th class="pb-2 font-medium text-gray-600">"100+"</th>
                                        <th class="pb-2 font-medium text-gray-600">"200+"</th>
                                        <th class="pb-2 font-medium text-gray-600">"250+"</th>
                                        <th class="pb-2 font-medium text-gray-600">"500+"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    <tr class="border-b border-gray-100">
                                        <td class="py-2 text-gray-600">"Total"</td>
                                        <td class="py-2 text-lg" title={achievements::TOTAL[5].name}>{achievements::TOTAL[5].emoji}</td>
                                        <td class="py-2 text-lg" title={achievements::TOTAL[4].name}>{achievements::TOTAL[4].emoji}</td>
                                        <td class="py-2 text-lg" title={achievements::TOTAL[3].name}>{achievements::TOTAL[3].emoji}</td>
                                        <td class="py-2 text-lg" title={achievements::TOTAL[2].name}>{achievements::TOTAL[2].emoji}</td>
                                        <td class="py-2 text-lg" title={achievements::TOTAL[1].name}>{achievements::TOTAL[1].emoji}</td>
                                        <td class="py-2"></td>
                                        <td class="py-2 text-lg" title={achievements::TOTAL[0].name}>{achievements::TOTAL[0].emoji}</td>
                                    </tr>
                                    <tr class="border-b border-gray-100">
                                        <td class="py-2 text-emerald-600">"Conversions"</td>
                                        <td class="py-2 text-lg" title={achievements::CONVERSIONS[3].name}>{achievements::CONVERSIONS[3].emoji}</td>
                                        <td class="py-2"></td>
                                        <td class="py-2 text-lg" title={achievements::CONVERSIONS[2].name}>{achievements::CONVERSIONS[2].emoji}</td>
                                        <td class="py-2 text-lg" title={achievements::CONVERSIONS[1].name}>{achievements::CONVERSIONS[1].emoji}</td>
                                        <td class="py-2"></td>
                                        <td class="py-2 text-lg" title={achievements::CONVERSIONS[0].name}>{achievements::CONVERSIONS[0].emoji}</td>
                                        <td class="py-2"></td>
                                    </tr>
                                    <tr>
                                        <td class="py-2 text-blue-600">"New Feedstocks"</td>
                                        <td class="py-2 text-lg" title={achievements::NEW_FEEDSTOCKS[3].name}>{achievements::NEW_FEEDSTOCKS[3].emoji}</td>
                                        <td class="py-2"></td>
                                        <td class="py-2 text-lg" title={achievements::NEW_FEEDSTOCKS[2].name}>{achievements::NEW_FEEDSTOCKS[2].emoji}</td>
                                        <td class="py-2 text-lg" title={achievements::NEW_FEEDSTOCKS[1].name}>{achievements::NEW_FEEDSTOCKS[1].emoji}</td>
                                        <td class="py-2"></td>
                                        <td class="py-2 text-lg" title={achievements::NEW_FEEDSTOCKS[0].name}>{achievements::NEW_FEEDSTOCKS[0].emoji}</td>
                                        <td class="py-2"></td>
                                    </tr>
                                </tbody>
                            </table>
                        </div>
                    </details>
                </div>
            </div>

            <div class="flex items-center text-xs font-semibold text-gray-500 uppercase tracking-wide mb-3">
                <span class="w-6 mr-1"></span>
                <span class="w-8">"#"</span>
                <span class="flex-1">"Contributor"</span>
                <span class="w-24 text-center flex items-center justify-center gap-1">
                    <ShapeIndicator contribution_type=ContributionType::Conversion />
                    "Conv"
                </span>
                <span class="w-24 text-center flex items-center justify-center gap-1">
                    <ShapeIndicator contribution_type=ContributionType::NewFeedstock />
                    "New"
                </span>
                <span class="w-16 text-right">"Total"</span>
            </div>

            <ul class="space-y-0">
                {contributors.into_iter().enumerate().map(|(index, contributor)| {
                    view! {
                        <ContributorRow index=index contributor=contributor />
                    }
                }).collect::<Vec<_>>()}
            </ul>

            <div class="mt-4 text-center">
                <p class="text-sm text-gray-400">
                    "Showing top 50 contributors. Data refreshed daily."
                </p>
            </div>
        </div>
    }.into_any()
}

#[component]
fn TopUnconvertedRanking(feedstocks: Vec<(String, u64, String)>) -> impl IntoView {
    if feedstocks.is_empty() {
        return view! {}.into_any();
    }

    // Take only the top 20 for display
    let top_feedstocks: Vec<_> = feedstocks.into_iter().take(20).collect();

    view! {
        <div class="bg-white rounded-lg p-8 shadow-sm border border-gray-200 hover:shadow-md transition-shadow duration-200">
            <div class="mb-6">
                <h2 class="text-2xl font-semibold text-gray-900 mb-2 tracking-tight">
                    "Ranking: Unconverted Feedstocks by Downloads"
                </h2>
                <p class="text-gray-500 leading-relaxed">
                    "Most downloaded feedstocks that haven't been converted to Recipe v1 yet. Migrate these to make a big impact :)"
                </p>
            </div>
            <div class="flex items-center text-xs font-semibold text-gray-500 uppercase tracking-wide mb-3">
                <span class="w-8">"#"</span>
                <span class="flex-1">"Feedstock Name"</span>
                <span class="w-24 text-right">"Downloads"</span>
            </div>
            <ul class="space-y-0">
                {top_feedstocks.into_iter().enumerate().map(|(index, (name, downloads, _recipe_type))| {
                    let github_url = format!("https://github.com/conda-forge/{}", name);
                    let display_name = name.replace("-feedstock", "");
                    let formatted_downloads = format_downloads(downloads);

                    view! {
                        <li>
                            <a
                                href=github_url
                                target="_blank"
                                rel="noopener noreferrer"
                                class="flex items-center py-2 -mx-2 px-2 rounded border-b border-dashed border-gray-200 hover:bg-gray-50 transition-colors duration-150 cursor-pointer"
                            >
                                <span class="w-8 text-sm font-medium text-gray-400 tabular-nums">
                                    {format!("#{}", index + 1)}
                                </span>
                                <span class="flex-1 font-medium text-blue-600">
                                    {display_name}
                                </span>
                                <span class="w-24 text-right text-sm font-medium text-gray-700 tabular-nums">
                                    {"~"}{formatted_downloads}
                                </span>
                            </a>
                        </li>
                    }
                }).collect::<Vec<_>>()}
            </ul>
            <div class="mt-4 text-center space-y-1">
                <p class="text-sm text-gray-400">
                    "Showing top 20 feedstocks."
                </p>
                <p class="text-sm text-gray-400">
                    "Download counts are summed across the 10 most recent versions."
                </p>
                <p class="text-sm text-gray-400">
                    "Data from "
                    <a href="https://prefix.dev/channels/conda-forge" target="_blank" rel="noopener noreferrer" class="text-blue-500 hover:underline">"prefix.dev"</a>
                    ", refreshed daily."
                </p>
            </div>
        </div>
    }.into_any()
}

fn main() {
    leptos::mount::mount_to_body(App)
}
