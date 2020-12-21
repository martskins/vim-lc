# VIM Language Client

This plugin is an LSP client implementation for neovim (vim8 not supported yet), heavily inspired by
`LanguageClient-neovim` (https://github.com/autozimu/LanguageClient-neovim). The project is still
in early development so use at your own risk :grin:.

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
## CONFIG

vim-lc has a rather basic configuration API, but to make it run properly you
must at least configure the language server, you can do that by adding a
dictionary under the variable `g:vlc#servers` which has filetypes as keys, and
server configuration as values:

```
let g:vlc#servers = {}
let g:vlc#servers.go = { 'command': 'gopls', name: 'gopls' }
let g:vlc#servers.rust = { 'command': 'rust-analyzer', name: 'rust-analyzer' }
```

As previously said, the values on the above map are server commands, the
commands have the following schema:

```
{
  name: String,
  command: String,
  args: [String]?,
  initializationOptions: Map?,
}
```

The initializationOptions field in the command is sent to the server upon
initialization, and it's value is specific to each server, so you should read
the server's documentation if you need to send something in it.

By default, vim-lc sets the log level to error and the log output to
`/tmp/vlc.log`, if you wish to change that you can do that by adding the
following to your vimrc:


```
let g:vlc#log#level = 'info'
let g:vlc#log#output = '/path/you/desire.log'
```

For a more complete configuration example see `minvimrc` in this repository.

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

vim-lc doesn't set any default mappings, but it provides plug mappings you can configure in your vimrc:

```
nmap <silent>gd <Plug>(vlc-definition)
nmap <silent>gi <Plug>(vlc-implementation)
nmap <silent>gr <Plug>(vlc-references)
nmap <silent>ga <Plug>(vlc-code-action)
nmap <silent>gl <Plug>(vlc-code-lens)
nmap <silent>R  <Plug>(vlc-rename)
nmap <silent>K  <Plug>(vlc-hover)
nmap <silent>F  <Plug>(vlc-formatting)
nmap <silent>E  <Plug>(vlc-diagnostic-detail)
```
