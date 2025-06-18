use leptos::prelude::*;

#[component]
fn App() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-white/80">
        <header class="text-center mb-10 bg-white/95 rounded-lg p-8 shadow-sm">
            <h1 class="text-4xl md:text-5xl font-bold text-gray-800 mb-4">
                "Conda-Forge: Are we rattler-build yet?"
            </h1>
            <p class="text-2xl text-gray-600 font-medium">
            </p>
        </header>
            <div class="max-w-7xl mx-auto px-4 py-8">

                <main class="bg-white/95 backdrop-blur-sm rounded-lg p-8 shadow-sm">
                </main>
            </div>
        </div>
    }
}

fn main() {
    leptos::mount::mount_to_body(App)
}
