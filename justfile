_:
  @just -l

_new-tmux-dev-session SESSION:
  tmux new -ds "{{SESSION}}" -n "README"
  tmux send-keys -t "{{SESSION}}":README 'nv ./README.md "+set wrap"' ENTER
  @just _new-window "{{SESSION}}" "Edit" ""
  # @just _new-window "{{SESSION}}" "Data Types" "cd midi-daw-types && nv src/lib.rs src/automation/{mod.rs,**/{mod.rs,*.rs}}"
  # @just _new-window "{{SESSION}}" "Edit Py" "cd python-lib && nv midi_daw/{main.py,__init__.py}"
  @just _new-window "{{SESSION}}" "Run" "just run"
  @just _new-window "{{SESSION}}" "Misc" ""
  @just _new-window "{{SESSION}}" "Git" "git status"

_new-window SESSION NAME CMD:
  tmux new-w -t "{{SESSION}}" -n "{{NAME}}"
  # tmux send-keys -t "{{SESSION}}":"{{NAME}}" ". ./.venv/bin/activate" ENTER
  [[ "{{CMD}}" != "" ]] && tmux send-keys -t "{{SESSION}}":"{{NAME}}" "{{CMD}}" ENTER || true

# _new-tmux-dev-session-2 SESSION:
#   tmux new -ds "{{SESSION}}" -n "Edit-1"
#   # tmux send-keys -t "{{SESSION}}":"Edit-1" '. ./.venv/bin/activate' ENTER
#   tmux send-keys -t "{{SESSION}}":"Edit-1" 'cd ./test-files/' ENTER
#   @just _new-window "{{SESSION}}" "Run-1" "cd ./test-files/"
#   @just _new-window "{{SESSION}}" "Edit-2" "cd ./test-files/"
#   @just _new-window "{{SESSION}}" "Run-2" "cd ./test-files/"
#   @just _new-window "{{SESSION}}" "Misc" "cd ./test-files/"

tmux:
  tmux has-session -t '=boxy-synth' || just _new-tmux-dev-session boxy-synth
  # tmux has-session -t '=midi-daw-test' || just _new-tmux-dev-session-2 midi-daw-test
  tmux a -t '=boxy-synth'

# tmux-2:
#   tmux has-session -t '=midi-daw-test' || just _new-tmux-dev-session-2 midi-daw-test
#   tmux a -t midi-daw-test

backup:
  git status
  git add .
  @just commit "backup commit"
  git push

commit message:
    git commit -m "{{message}}"
    git push

commit-all message:
    git commit -am "{{message}}"
    git push

run:
  WEBKIT_DISABLE_COMPOSITING_MODE=1 dx serve --platform desktop
