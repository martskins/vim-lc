use super::VLC;
use crate::vim::*;
use crate::VIM;
use failure::Fallible;

impl VLC {
    pub async fn show_locations(&self, input: Vec<Location>) -> Fallible<()> {
        if input.is_empty() {
            return Ok(());
        }

        if input.len() == 1 {
            return self.jump_to_location(input.first().cloned().unwrap()).await;
        }

        let pwd = std::env::current_dir()?;
        let pwd = format!("file://{}/", pwd.to_str().unwrap());
        let list = input
            .into_iter()
            .map(|l| QuickfixItem {
                bufnr: 0,
                filename: l.filename.replace(pwd.as_str(), ""),
                lnum: l.line,
                col: l.col,
                text: String::new(),
                kind: 'W',
            })
            .collect();

        self.set_quickfix(list).await?;
        Ok(())
    }

    pub async fn jump_to_location(&self, input: Location) -> Fallible<()> {
        let command = format!("cursor({}, {})", input.line, input.col);
        self.call(EvalParams { command }).await?;
        Ok(())
    }

    pub async fn call(&self, cmd: EvalParams) -> Fallible<()> {
        let mut client = VIM.clone().client;
        client.call("call", cmd).await?;
        Ok(())
    }

    async fn set_quickfix(&self, list: Vec<QuickfixItem>) -> Fallible<()> {
        let mut client = VIM.clone().client;
        client.notify("setQuickfix", list).await?;
        self.command(vec!["copen"]).await?;
        Ok(())
    }

    async fn command(&self, cmd: Vec<&str>) -> Fallible<()> {
        let mut client = VIM.clone().client;
        client.notify("command", cmd).await?;
        Ok(())
    }

    pub async fn show_message(&self, message: Message) -> Fallible<()> {
        let mut client = VIM.clone().client;
        client.notify("showMessage", message).await?;
        Ok(())
    }

    pub async fn log_message(&self, params: lsp_types::LogMessageParams) -> Fallible<()> {
        log::debug!("{}", params.message);
        Ok(())
    }
}
