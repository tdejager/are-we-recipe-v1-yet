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
- **Purpose**: Analyzes conda-forge repositories via GitHub API
- **Output**: Generates `feedstock-stats.toml` with current statistics
- **Automation**: Runs daily via GitHub Actions

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
pixi run data-collection  # Run data collector (requires GITHUB_TOKEN)
```

### Environment Setup

For data collection, create a `.env` file:

```bash
GITHUB_TOKEN=your_github_personal_access_token_here
```

## 📊 Data Collection

The data collector:
1. Searches for conda-forge feedstock repositories
2. Analyzes each repository's structure to detect recipe format
3. Categorizes feedstocks as: Recipe v1, meta.yaml, or Unknown
4. Outputs statistics to `feedstock-stats.toml`

Categories:
- **Recipe v1**: Repositories with `recipe/recipe.yaml`
- **meta.yaml**: Repositories with `recipe/meta.yaml`
- **Unknown**: Repositories with neither, both, or inaccessible

## 🤖 Automation

Two GitHub Actions workflows handle automation:

### Data Collection (`data-collection.yml`)
- **Schedule**: Daily at 2 AM UTC
- **Purpose**: Updates feedstock statistics
- **Trigger**: Scheduled + manual dispatch

### Deployment (`deploy.yml`)
- **Trigger**: Push to main branch (when web files or stats change)
- **Purpose**: Builds and deploys to GitHub Pages
- **Output**: Static site at `https://your-username.github.io/are-we-recipe-v1-yet/`

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

Built with ❤️ by the conda community • Powered by [Leptos](https://leptos.dev/) and [rattler-build](https://rattler.build)
