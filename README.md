# VIM Language Client

This plugin is an LSP client implementation for neovim (vim8 not supported yet), heavily inspired in
`LanguageClient-neovim` (https://github.com/autozimu/LanguageClient-neovim). The project is still
in early development so use at your own risk :grin:.

## DEPENDENCIES

vim-lc uses `fzf` to show references, implementation, and other commands that result in a list
showing. If you don't have `fzf` installed, you should install it by following the instructions
found in: https://github.com/junegunn/fzf.vim#installation

In addition to this, vim-lc is developed on top neovim, so it expects things like `:terminal` to be
available. This is currently used for code lens actions.

## INSTALLATION

If you are using vim-plug you can install vim-lc by adding the following line to your `.vimrc`.

```
Plug 'martskins/vim-lc', { 'do': 'cargo build --release' }
```

That will download and compile the plugin. This plugin is developed in Rust, so you must have Rust
installed in your system in order to compile it. There are plans to provide pre-compiled binaries
for most platforms in the near future so this step shouldn't be necessary after that.

The default installation of vim-lc assumes that the executable will be found at `~/.vim/plugged/vim-lc/target/release/vlc`.
If you have installed the plugin, or placed the binary in another folder, you can use the `binpath`
config option by adding this line in your `.vimrc`.

```
let g:vlc#binpath = '~/path/to/vlc/binary'
```

vim-lc uses a toml config file for the client's config options. On startup it will try to locate the
file under `~/.vlc/config.toml`. If you want to use a different path for your config file you can
use the following config value.

```
let g:vlc#config = '~/path/to/vlc/config'
```

## CONFIG

As mentioned before, vim-lc uses a `toml` file as a way to configure the client.

The config file has four root levels configs, although all of them are optional. These root level
configs are `log`, `servers`, `diagnostics`, `completion`.

The available options for each of them are:

```toml
[log]
level = "debug"     # one of 'debug', 'warn', 'info', 'error'
output = "~/.vlc/log.txt"

[diagnostics]
enabled = true
auto_open = true    # if set to true, opens quickfix list after populating it with diagnostics
show_signs = false  # if set to true it shows gutter signs for diagnostics

[completion]
enabled = true
strategy = "ncm2"   # one of "ncm2" or "omnifunc"

[servers]           # each item under this key has the form of `{filetype}: {lsp_server_binary}`
go = "gopls"
rust = "ra_lsp_server"
```

## COMMANDS

**VLCDefintion**:       jumps to the definition of the symbol under the cursor.

**VLCImplementation**:  lists the implementation of the symbol under the cursor or jumps to it if there is a single implementation.

**VLCReferences**:      shows a list of references for the symbol under the cursor. If there a single reference then it jumps to it instead of showing a list.

**VLCCodeAction**:      shows code actions for the symbol under the cursor.

**VLCCodeLensAction**:  when called on a line with a visible code lens, it will show a list with the possible code lens actions for that line.

**VLCRename**:          rename the symbol under the cursor.

**VLCHover**:           shows documentation for the symbol under the cursor.

**VLCStop**:            stops the language server for the filetype of the active buffer.

**VLCStart**:           starts the language server for the filetype of the active buffer.

## MAPPINGS

vim-lc doesn't set any default mappings, but you can use these suggested mappings:

```
nnoremap <silent>gl         :call vlc#codeLensAction()<CR>
nnoremap <silent>ga         :call vlc#codeAction()<CR>
vnoremap <silent>ga         :call vlc#codeAction()<CR>
nnoremap <silent>gd         :call vlc#goToDefinition()<CR>
nnoremap <silent>K          :call vlc#hover()<CR>
nnoremap <silent>R          :call vlc#rename()<CR>
nnoremap <silent>gr         :call vlc#references()<CR>
nnoremap <silent>gi         :call vlc#implementation()<CR>
nnoremap <c-s>              :call vlc#start()<CR>
nnoremap <c-k>              :call vlc#stop()<CR>

```
