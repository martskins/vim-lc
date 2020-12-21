call call('vlc#run', [])

call sign_define('VLCSignError', {
  \ 'text' : '!!',
  \ 'texthl' : 'Error' })

call sign_define('VLCSignWarn', {
  \ 'text' : '!',
  \ 'texthl' : 'Warn'})

call sign_define('VLCSignInfo', {
  \ 'text' : '>>',
  \ 'texthl' : 'Info'})

call sign_define('VLCSignHint', {
  \ 'text' : '>',
  \ 'texthl' : 'Hint'})

nnoremap <Plug>(vlc-formatting)         :call vlc#formatting()<CR>
nnoremap <Plug>(vlc-definition)         :call vlc#definition()<CR>
nnoremap <Plug>(vlc-implementation)     :call vlc#implementation()<CR>
nnoremap <Plug>(vlc-references)         :call vlc#references()<CR>
nnoremap <Plug>(vlc-code-action)        :call vlc#code_action()<CR>
nnoremap <Plug>(vlc-code-lens)          :call vlc#code_lens_action()<CR>
nnoremap <Plug>(vlc-rename)             :call vlc#rename()<CR>
nnoremap <Plug>(vlc-hover)              :call vlc#hover()<CR>
nnoremap <Plug>(vlc-shutdown)           :call vlc#shutdown()<CR>
nnoremap <Plug>(vlc-start)              :call vlc#start()<CR>
nnoremap <Plug>(vlc-diagnostic-detail)  :call vlc#diagnostic_detail()<CR>

function! s:configure()
  if !vlc#has_server_configured(&filetype)
    return
  endif

  set omnifunc=vlc#completion

  command! VLCFormatting          call vlc#formatting()
  command! VLCDefinition          call vlc#definition()
  command! VLCImplementation      call vlc#implementation()
  command! VLCReferences          call vlc#references()
  command! VLCCodeAction          call vlc#code_action()
  command! VLCCodeLensAction      call vlc#code_lens_action()
  command! VLCRename              call vlc#rename()
  command! VLCHover               call vlc#hover()
  command! VLCStop                call vlc#shutdown()
  command! VLCStart               call vlc#start()
  command! VLCDiagnosticDetail    call vlc#diagnostic_detail()

  augroup vlc
      autocmd!
      autocmd TextChanged   <buffer> call vlc#lsp#did_change()
      autocmd BufWritePost  <buffer> call vlc#lsp#did_save()
      autocmd BufWinLeave   <buffer> call vlc#lsp#did_close()
      autocmd VimLeavePre   <buffer> call vlc#lsp#exit()
      autocmd TextChangedP  <buffer> call vlc#lsp#did_change()
      autocmd TextChangedI  <buffer> call vlc#lsp#did_change()

      autocmd CompleteDone  <buffer> call vlc#resolve_completion()
      autocmd InsertCharPre <buffer> call vlc#trigger_completion()
  augroup END

  call vlc#lsp#did_open()
endfunction

augroup vlc_init
  autocmd!
  autocmd FileType * call s:configure()
  autocmd BufEnter * call s:configure()
augroup END
