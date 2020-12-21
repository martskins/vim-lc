function! vlc#ncm2#register(params) abort
  let l:complete_pattern = a:params['complete_pattern']
  let l:cpp = []
  for cp in l:complete_pattern
    let l:cpp = add(l:cpp, escape(cp, '.\/:'))
  endfor
  call ncm2#register_source({
      \ 'name' : 'vlc',
      \ 'scope': [a:params['language_id']],
      \ 'priority': 9,
      \ 'mark': 'VLC',
      \ 'subscope_enable': 1,
      \ 'complete_length': -1,
      \ 'complete_pattern': l:cpp,
      \ 'on_complete': ['vlc#ncm2#completion'],
      \ })
endfunction

" ncm2 completion func
function! vlc#ncm2#completion(ctx) abort
  call vlc#lsp#completion(funcref('vlc#ncm2#do_complete', [a:ctx]))
endfunction

" ncm2 completion callback to populate completion list
function! vlc#ncm2#do_complete(ctx, res) abort
  call ncm2#complete(a:ctx, col('.'), a:res['words'], 0)
endfunction
