use leptos::prelude::*;

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

    // Helper function to format ISO date to human readable
    let format_date = |iso_date: &str| -> String {
        if let Some(date_part) = iso_date.split('T').next() {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
                return date.format("%b %d, %Y").to_string();
            }
        }
        iso_date.to_string()
    };

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

#[component]
fn TopUnconvertedRanking(feedstocks: Vec<(String, u64, String)>) -> impl IntoView {
    if feedstocks.is_empty() {
        return view! {}.into_any();
    }

    // Helper function to format download counts
    let format_downloads = |count: u64| -> String {
        if count >= 1_000_000 {
            format!("{:.1}M", count as f64 / 1_000_000.0)
        } else if count >= 1_000 {
            format!("{:.1}K", count as f64 / 1_000.0)
        } else {
            count.to_string()
        }
    };

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
            <div class="mt-4 text-center">
                <p class="text-sm text-gray-400">
                    "Showing top 20 feedstocks. Data refreshed daily."
                </p>
            </div>
        </div>
    }.into_any()
}

fn main() {
    leptos::mount::mount_to_body(App)
}
