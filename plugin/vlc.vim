call call('vim#start', [])

set omnifunc=vlc#completion

command! VLCFormatting      call vlc#formatting()
command! VLCDefinition      call vlc#definition()
command! VLCImplementation  call vlc#implementation()
command! VLCReferences      call vlc#references()
command! VLCCodeAction      call vlc#code_action()
command! VLCCodeLensAction  call vlc#code_lens_action()
command! VLCRename          call vlc#rename()
command! VLCHover           call vlc#hover()
command! VLCStop            call vlc#shutdown()
command! VLCStart           call vim#start()

call sign_define('vlc_error', {
  \ 'text' : '!!',
  \ 'texthl' : 'Error' })

call sign_define('vlc_warn', {
  \ 'text' : '--',
  \ 'texthl' : 'Warn'})

augroup vlc
    autocmd!
    autocmd FileType      *   call vlc#did_open()
    autocmd TextChanged   *   call lsp#did_change()
    autocmd BufWritePost  *   call lsp#did_save()
    autocmd BufWinLeave   *   call lsp#did_close()
    autocmd VimLeavePre   *   call lsp#exit()
    autocmd TextChangedP  *   call lsp#did_change()
    autocmd TextChangedI  *   call lsp#did_change()
    autocmd CompleteDone  *   call vlc#resolve_completion()
augroup END
