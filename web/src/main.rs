use leptos::prelude::*;

#[component]
fn App() -> impl IntoView {
    let stats = include_str!("stats.toml");
    let toml_data: toml::Table = toml::from_str(stats).unwrap();
    
    let converted_recipes = toml_data.get("recipe_v1_count").unwrap().as_integer().unwrap() as u32;
    let total_recipes = toml_data.get("total_feedstocks").unwrap().as_integer().unwrap() as u32;
    let percentage = (converted_recipes as f64 / total_recipes as f64 * 100.0) as u32;

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
    let converted_angle = percentage * 3.6;

    view! {
        <div class="flex flex-col items-center">
            <h2 class="text-2xl font-semibold text-gray-900 mb-8">Migration Progress</h2>
            <div class="relative w-64 h-64">
                <svg class="w-full h-full transform -rotate-90" viewBox="0 0 200 200">
                    <circle
                        cx="100"
                        cy="100"
                        r="80"
                        fill="none"
                        stroke="#e5e7eb"
                        stroke-width="20"
                    />
                    <circle
                        cx="100"
                        cy="100"
                        r="80"
                        fill="none"
                        stroke="#F9C500"
                        stroke-width="20"
                        stroke-dasharray=format!("{} {}", converted_angle * 1.396, 502.65 - converted_angle * 1.396)
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

fn main() {
    leptos::mount::mount_to_body(App)
}
