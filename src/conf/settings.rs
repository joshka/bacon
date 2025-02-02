use {
    crate::*,
    anyhow::{
        Result,
        bail,
    },
    std::{
        collections::HashMap,
        path::PathBuf,
        time::Duration,
    },
};

/// The settings used in the application.
///
/// They're made from default, overriden (in order)
/// by the general prefs (global prefs.toml file), by
/// the package config (bacon.toml file in the project
/// directory) and by the launch arguments.
///
/// They're immutable during the execution of the missions.
#[derive(Debug, Clone)]
pub struct Settings {
    pub additional_alias_args: Option<Vec<String>>,
    pub additional_job_args: Vec<String>,
    pub all_features: bool,
    pub arg_job: Option<ConcreteJobRef>,
    /// Path of the files which were used to build the settings
    /// (note that not all settings come from files)
    pub config_files: Vec<PathBuf>,
    pub default_job: ConcreteJobRef,
    pub default_watch: bool,
    pub exports: ExportsSettings,
    pub features: Option<String>, // comma separated list
    pub grace_period: Period,
    pub help_line: bool,
    pub ignore: Vec<String>,
    pub ignored_lines: Option<Vec<LinePattern>>,
    pub jobs: HashMap<String, Job>,
    pub keybindings: KeyBindings,
    pub no_default_features: bool,
    pub on_change_strategy: Option<OnChangeStrategy>,
    pub reverse: bool,
    pub show_changes_count: bool,
    pub summary: bool,
    pub watch: Vec<String>,
    pub wrap: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            arg_job: Default::default(),
            additional_job_args: Default::default(),
            additional_alias_args: Default::default(),
            summary: false,
            wrap: true,
            reverse: false,
            help_line: true,
            no_default_features: Default::default(),
            all_features: Default::default(),
            features: Default::default(),
            keybindings: Default::default(),
            jobs: Default::default(),
            default_job: Default::default(),
            exports: Default::default(),
            show_changes_count: false,
            on_change_strategy: None,
            ignore: Default::default(),
            ignored_lines: Default::default(),
            grace_period: Duration::from_millis(5).into(),
            config_files: Default::default(),
            default_watch: true,
            watch: Default::default(),
        }
    }
}

impl Settings {
    /// Read the settings from all configuration files and arguments.
    ///
    ///
    /// Hardcoded defaults are overriden by the following configuration elements, in order:
    /// * the global `prefs.toml`
    /// * the file whose path is in environment variable `BACON_PREFS`
    /// * the workspace level `bacon.toml` file
    /// * the package level `bacon.toml` file
    /// * the file whose path is in environment variable `BACON_CONFIG`
    /// * args given as arguments, coming from the cli call
    pub fn read(
        args: &Args,
        context: &Context,
    ) -> Result<Self> {
        let mut settings = Settings::default();

        let default_package_config = Config::default_package_config();
        settings.apply_config(&default_package_config);

        if let Some(prefs_path) = bacon_prefs_path() {
            if prefs_path.exists() {
                let prefs = Config::from_path(&prefs_path)?;
                info!("prefs: {:#?}", &prefs);
                settings.register_config_file(prefs_path);
                settings.apply_config(&prefs);
            }
        }

        if let Some(config_path) = config_path_from_env("BACON_PREFS") {
            let config = Config::from_path(&config_path)?;
            info!("config from env: {:#?}", &config);
            settings.register_config_file(config_path);
            settings.apply_config(&config);
        }

        let workspace_config_path = context.workspace_config_path();
        let package_config_path = context.package_config_path();

        if let Some(workspace_config_path) = workspace_config_path {
            if workspace_config_path.exists() {
                info!("loading workspace level bacon.toml");
                let workspace_config = Config::from_path(&workspace_config_path)?;
                settings.register_config_file(workspace_config_path);
                settings.apply_config(&workspace_config);
            }
        }

        if package_config_path.exists() {
            let config = Config::from_path(&package_config_path)?;
            settings.register_config_file(package_config_path);
            settings.apply_config(&config);
        }

        if let Some(config_path) = config_path_from_env("BACON_CONFIG") {
            let config = Config::from_path(&config_path)?;
            info!("config from env: {:#?}", &config);
            settings.register_config_file(config_path);
            settings.apply_config(&config);
        }

        settings.apply_args(args);
        settings.check()?;
        info!("settings: {:#?}", &settings);
        Ok(settings)
    }

    pub fn register_config_file(
        &mut self,
        path: PathBuf,
    ) {
        self.config_files.push(path);
    }

    /// Apply one of the configuration elements, overriding
    /// defaults and previously applied configuration elements
    pub fn apply_config(
        &mut self,
        config: &Config,
    ) {
        if let Some(b) = config.summary {
            self.summary = b;
        }
        if let Some(b) = config.wrap {
            self.wrap = b;
        }
        if let Some(b) = config.reverse {
            self.reverse = b;
        }
        if let Some(b) = config.help_line {
            self.help_line = b;
        }
        #[allow(deprecated)] // for compatibility
        if config.vim_keys == Some(true) {
            self.keybindings.add_vim_keys();
        }
        if let Some(keybindings) = config.keybindings.as_ref() {
            self.keybindings.add_all(keybindings);
        }
        if config.additional_alias_args.is_some() {
            self.additional_alias_args
                .clone_from(&config.additional_alias_args);
        }
        for (name, job) in &config.jobs {
            self.jobs.insert(name.clone(), job.clone());
        }
        if let Some(default_job) = &config.default_job {
            self.default_job = default_job.clone();
        }
        self.exports.apply_config(config);
        if let Some(b) = config.show_changes_count {
            self.show_changes_count = b;
        }
        if let Some(b) = config.on_change_strategy {
            self.on_change_strategy = Some(b);
        }
        if let Some(b) = config.ignored_lines.as_ref() {
            self.ignored_lines = Some(b.clone());
        }
        if let Some(p) = config.grace_period {
            self.grace_period = p;
        }
        if let Some(b) = config.default_watch {
            self.default_watch = b;
        }
        if let Some(watch) = config.watch.as_ref() {
            self.watch = watch.clone();
        }
        for pattern in &config.ignore {
            self.ignore.push(pattern.clone());
        }
    }
    pub fn apply_args(
        &mut self,
        args: &Args,
    ) {
        if let Some(job) = &args.job {
            self.arg_job = Some(job.clone());
        }
        if args.no_summary {
            self.summary = false;
        }
        if args.summary {
            self.summary = true;
        }
        if args.no_wrap {
            self.wrap = false;
        }
        if args.wrap {
            self.wrap = true;
        }
        if args.no_reverse {
            self.reverse = false;
        }
        if args.help_line {
            self.help_line = true;
        }
        if args.no_help_line {
            self.help_line = false;
        }
        if args.export_locations {
            self.exports.set_locations_export_auto(true);
        }
        if args.no_export_locations {
            self.exports.set_locations_export_auto(false);
        }
        if args.reverse {
            self.reverse = true;
        }
        if args.no_default_features {
            self.no_default_features = true;
        }
        if args.all_features {
            self.all_features = true;
        }
        if args.features.is_some() {
            self.features.clone_from(&args.features);
        }
        self.additional_job_args
            .clone_from(&args.additional_job_args);
    }

    pub fn check(&self) -> Result<()> {
        if self.jobs.is_empty() {
            bail!("Invalid configuration : no job found");
        }
        if let NameOrAlias::Name(name) = &self.default_job.name_or_alias {
            if !self.jobs.contains_key(name) {
                bail!("Invalid configuration : default job ({name:?}) not found in jobs");
            }
        }
        Ok(())
    }
}
