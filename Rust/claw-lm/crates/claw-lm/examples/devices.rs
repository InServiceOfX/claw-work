fn main() -> anyhow::Result<()> {
    #[cfg(feature = "cuda")]
    {
        println!(
            "{}",
            serde_json::to_string_pretty(&claw_lm::cuda::devices()?)?
        );
        Ok(())
    }
    #[cfg(not(feature = "cuda"))]
    {
        anyhow::bail!("Build with --features cuda")
    }
}
