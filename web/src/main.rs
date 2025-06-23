use leptos::prelude::*;

#[component]
fn App() -> impl IntoView {
    let stats = include_str!("stats.toml");
    let toml_data: toml::Table = toml::from_str(stats).unwrap();
    
    let converted_recipes = toml_data.get("recipe_v1_count").unwrap().as_integer().unwrap() as u32;
    let total_recipes = toml_data.get("total_feedstocks").unwrap().as_integer().unwrap() as u32;
    let percentage = (converted_recipes as f64 / total_recipes as f64 * 100.0) as u32;
    
    let recently_updated = toml_data.get("recently_updated")
        .and_then(|v| v.as_table())
        .map(|table| {
            table.iter().map(|(name, date)| {
                (name.clone(), date.as_str().unwrap_or("").to_string())
            }).collect::<Vec<_>>()
        })
        .unwrap_or_default();
    
    let last_updated = toml_data.get("last_updated")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    view! {
        <div class="min-h-screen bg-gray-50">
            <header class="text-center py-16 px-4">
                <h1 class="text-5xl md:text-6xl font-bold text-gray-900 mb-6">
                    "Are we recipe v1 yet?"
                </h1>
                <p class="text-xl text-gray-600 max-w-3xl mx-auto mb-8">
                    "Tracking the progress of migrating conda-forge recipes from the legacy meta.yaml format to the new recipe.yaml format"
                </p>
                <div class="max-w-4xl mx-auto text-left bg-white rounded-lg p-8 shadow-sm">
                    <h2 class="text-2xl font-semibold text-gray-800 mb-4">"What is Recipe v1?"</h2>
                    <p class="text-gray-700 mb-4">
                        "Recipe v1 is the new standardized format for conda package recipes, replacing the legacy meta.yaml format. "
                        "This new format provides better structure, validation, and tooling support for creating conda packages."
                    </p>
                    <p class="text-gray-700">
                        "Learn more about the specification in "
                        <a href="https://github.com/conda/ceps/blob/main/cep-0013.md" class="text-blue-600 hover:text-blue-800 underline">"CEP-0013"</a>
                        " and the migration process in "
                        <a href="https://github.com/conda/ceps/blob/main/cep-0014.md" class="text-blue-600 hover:text-blue-800 underline">"CEP-0014"</a>
                        "."
                    </p>
                    <p class="text-gray-700">
                      "You can use "<a href="rattler.build" class="hover:underline text-blue-800">"rattler-build"</a>
                      " to build recipes in the new format."
                    </p>
                </div>
            </header>
            <div class="max-w-6xl mx-auto px-4 pb-8">
                <main class="bg-white rounded-lg p-8 shadow-sm border border-gray-200">
                    <div class="grid md:grid-cols-2 gap-12 items-center">
                        <MigrationChart converted=converted_recipes total=total_recipes />
                        <MigrationStats converted=converted_recipes total=total_recipes percentage=percentage />
                    </div>
                </main>
                <div class="mt-8">
                    <RecentlyUpdated feedstocks=recently_updated last_updated=last_updated.to_string() />
                </div>
            </div>
            <div class="mx-4 mt-8 mb-6">
            <a class="" href="https://rattler.build" target="_blank">
                <img
                    src="./banner.png"
                    alt="rattler-build: A fast package build tool for Conda packages written in Rust"
                    class="w-full max-w-4xl mx-auto rounded-lg shadow-lg hover:scale-105 transition-transform duration-300"
                />
            </a>
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

    view! {
        <div class="flex flex-col items-center">
            <h2 class="text-2xl font-semibold text-gray-900 mb-8">Migration Progress</h2>
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
                        stroke-dasharray=format!("{:.2} {:.2}", arc_length, remaining_length)
                        stroke-linecap="round"
                        class="transition-all duration-1000 ease-out"
                    />
                </svg>
                <div class="absolute inset-0 flex items-center justify-center">
                    <div class="text-center">
                        <div class="text-3xl font-bold text-gray-900">{format!("{:.1}%", percentage)}</div>
                        <div class="text-sm text-gray-600">Complete</div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn MigrationStats(converted: u32, total: u32, percentage: u32) -> impl IntoView {
    view! {
        <div class="space-y-8">
            <div class="text-center">
                <h2 class="text-2xl font-semibold text-gray-900 mb-4">Migration Statistics</h2>
                <div class="text-5xl font-bold text-emerald-600 mb-2">
                    {converted.to_string()}
                </div>
                <div class="text-lg text-gray-600 mb-4">
                    "out of " {total.to_string()} " recipes converted"
                </div>
                <div class="w-full bg-gray-200 rounded-full h-4 mb-6">
                    <div
                        class="bg-[#F9C500] h-4 rounded-full transition-all duration-1000 ease-out"
                        style=format!("width: {}%", percentage)
                    ></div>
                </div>
            </div>

            <div class="grid grid-cols-2 gap-4">
                <div class="bg-emerald-50 rounded-lg p-4 text-center border border-emerald-200">
                    <div class="text-2xl font-bold text-emerald-700">{converted.to_string()}</div>
                    <div class="text-sm text-emerald-600">Converted</div>
                </div>
                <div class="bg-gray-50 rounded-lg p-4 text-center border border-gray-200">
                    <div class="text-2xl font-bold text-gray-700">{(total - converted).to_string()}</div>
                    <div class="text-sm text-gray-600">Remaining</div>
                </div>
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
        <div class="bg-white rounded-lg p-8 shadow-sm border border-gray-200">
            <div class="flex items-center justify-between mb-4">
                <h2 class="text-lg font-medium text-gray-800">Recently Updated to Recipe v1</h2>
                <span class="text-xs text-gray-400">"Updated " {formatted_date}</span>
            </div>
            <div class="flex items-center text-xs font-bold text-gray-500 uppercase tracking-wide mb-3">
                <span>Recipe Name</span>
                <span class="flex-1"></span>
                <span>Change Detected</span>
            </div>
            <ul class="space-y-2">
                {feedstocks.into_iter().map(|(name, date)| {
                    let formatted_date = format_date(&date);
                    let github_url = format!("https://github.com/conda-forge/{}", name);
                    view! {
                        <li class="flex items-center text-gray-700">
                            <a 
                                href=github_url
                                target="_blank"
                                rel="noopener noreferrer"
                                class="font-medium text-blue-600 hover:text-blue-800 hover:underline"
                            >
                                {name.replace("-feedstock", "")}
                            </a>
                            <span class="flex-1 border-b border-dotted border-gray-300 mx-3"></span>
                            <span class="text-sm text-gray-600">{formatted_date}</span>
                        </li>
                    }
                }).collect::<Vec<_>>()}
            </ul>
        </div>
    }.into_any()
}

fn main() {
    leptos::mount::mount_to_body(App)
}
