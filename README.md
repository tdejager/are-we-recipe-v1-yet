# Are we Recipe v1 yet? 🍃

A website tracking the progress of migrating conda-forge recipes from the legacy `meta.yaml` format to the new `recipe.yaml` format (Recipe v1).

## 🌟 About

This project monitors the adoption of [Recipe v1](https://github.com/conda/ceps/blob/main/cep-0013.md), the new standardized format for conda package recipes. Recipe v1 provides better structure, validation, and tooling support compared to the legacy `meta.yaml` format.

## 🏗️ Architecture

The project consists of two main components:

### Web Frontend (`web/`)
- **Framework**: [Leptos](https://leptos.dev/) with client-side rendering
- **Styling**: Tailwind CSS v4 with Inter font
- **Build**: Trunk for WASM compilation

### Data Collector (`data-collector/`)
- **Purpose**: Analyzes conda-forge feedstocks via cf-graph-countyfair sparse checkout
- **Output**: Generates `feedstock-stats.toml` with current statistics
- **Method**: Uses git sparse checkout for efficient metadata access

## 🚀 Development

### Prerequisites
- [pixi](https://pixi.sh/) package manager

### Quick Start

```bash
# Clone the repository
git clone https://github.com/your-username/are-we-recipe-v1-yet.git
cd are-we-recipe-v1-yet

# Install dependencies
pixi install

# Start development server
pixi run dev
# Opens http://localhost:8080
```

### Available Commands

```bash
# Development
pixi run dev           # Start dev server with hot reload

# Building
pixi run build         # Production build

# Data Collection
pixi run collect-data         # Run data collector
pixi run collect-data-fresh   # Force fresh sparse checkout
pixi run collect-data-verbose # Run with verbose output
```

## 📊 Data Collection

The data collector:
1. Uses git sparse checkout to download cf-graph-countyfair metadata
2. Analyzes ~26k feedstock JSON files efficiently
3. Detects Recipe v1 by checking `conda_build_tool: "rattler-build"` in conda-forge.yml
4. Outputs statistics to `feedstock-stats.toml`

Categories:
- **Recipe v1**: Feedstocks using rattler-build
- **meta.yaml**: Feedstocks using conda-build
- **Unknown**: Feedstocks with no clear build tool specified

## 🤖 Automation

GitHub Actions workflows handle daily data collection and deployment. The data collector uses sparse checkout for efficient CI/CD execution.

## 📚 Learn More

- [Recipe v1 Specification (CEP-0013)](https://github.com/conda/ceps/blob/main/cep-0013.md)
- [Recipe v1 Migration (CEP-0014)](https://github.com/conda/ceps/blob/main/cep-0014.md)
- [rattler-build](https://rattler.build) - Tool for building Recipe v1 packages

## 🤝 Contributing

Contributions are welcome! Please feel free to:
- Report bugs or request features via GitHub Issues
- Submit pull requests for improvements
- Suggest design enhancements

---

Built with ❤️ by the conda community • Powered by [Leptos](https://leptos.dev/) and [pixi](https://pixi.sh/)
