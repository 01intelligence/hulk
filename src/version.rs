/// Returs the Hulk version information.
pub fn hulk_version_info(build_time: Option<&str>) -> String {
    let fallback = "Unknown (env var does not exist when building)";
    format!(
        "\nRelease Version:   {}\
         \nEdition:           {}\
         \nGit Commit Hash:   {}\
         \nGit Commit Branch: {}\
         \nUTC Build Time:    {}\
         \nRust Version:      {}\
         \nEnable Features:   {}\
         \nProfile:           {}",
        env!("CARGO_PKG_VERSION"),
        option_env!("HULK_EDITION").unwrap_or("Community"),
        option_env!("HULK_BUILD_GIT_HASH").unwrap_or(fallback),
        option_env!("HULK_BUILD_GIT_BRANCH").unwrap_or(fallback),
        build_time.unwrap_or(fallback),
        option_env!("HULK_BUILD_RUSTC_VERSION").unwrap_or(fallback),
        option_env!("HULK_ENABLE_FEATURES")
            .unwrap_or(fallback)
            .trim(),
        option_env!("HULK_PROFILE").unwrap_or(fallback),
    )
}
