#!/usr/bin/env bash
# ui.sh — caixas do assistente tuiOS. UI=1 → dialog; UI=0 → texto puro (--auto).
UI="${UI:-1}"
UI_LOG="${UI_LOG:-/var/log/tuios-install.log}"

ui_log() { echo "[$(date '+%F %T')] $*" >> "$UI_LOG" 2>/dev/null || true; }

_ui_dialog() { [ "${UI:-0}" = 1 ] && command -v dialog >/dev/null 2>&1; }

ui_msg() { # $1=titulo $2=texto (aceita \n)
  ui_log "MSG $1"
  if _ui_dialog; then dialog --title "$1" --msgbox "$(printf '%b' "$2")" 14 74
  else printf '\n== %s ==\n%b\n' "$1" "$2"; fi
}

ui_erro() { # $1=titulo $2=texto — sempre reporta
  ui_log "ERRO $1: $2"
  if _ui_dialog; then dialog --title "ERRO — $1" --msgbox "$(printf '%b' "$2")" 16 74
  else printf '\n!! ERRO — %s\n%b\n' "$1" "$2" >&2; fi
}

ui_yesno() { # $1=titulo $2=texto; 0=sim
  if _ui_dialog; then dialog --title "$1" --yesno "$(printf '%b' "$2")" 14 74
  else printf '%b [s/N]: ' "$2"; local r; read -r r; case "$r" in s|S|sim) return 0;; *) return 1;; esac; fi
}

ui_confirma_texto() { # $1=titulo $2=texto $3=literal; 0=só se digitou o literal
  local resp
  if _ui_dialog; then
    resp=$(dialog --title "$1" --inputbox "$(printf '%b' "$2")\n\nDigite $3 para confirmar:" 16 74 3>&1 1>&2 2>&3) || return 1
  else
    printf '%b\nDigite %s: ' "$2" "$3"; read -r resp
  fi
  [ "$resp" = "$3" ]
}

ui_menu() { # $1=titulo $2=texto; $3...="tag|descrição" → echo da tag (vazio=cancelou)
  if _ui_dialog; then
    local args=() par
    for par in "${@:3}"; do args+=("${par%%|*}" "${par#*|}"); done
    dialog --title "$1" --menu "$(printf '%b' "$2")" 22 76 "${#args[@]}" "${args[@]}" 3>&1 1>&2 2>&3 || true
  else
    printf '\n== %s ==\n%b\n' "$1" "$2"
    local i=1 par op
    for par in "${@:3}"; do printf ' %2d) %s\n' "$i" "${par#*|}"; i=$((i+1)); done
    read -rp "escolha [1-$((i-1))]: " op
    par=$(printf '%s\n' "${@:3}" | sed -n "${op:-0}p") || true
    printf '%s\n' "${par%%|*}"
  fi
}

ui_senha() { # $1=titulo $2=texto → echo senha
  if _ui_dialog; then
    dialog --title "$1" --insecure-input --passwordbox "$(printf '%b' "$2")" 10 60 3>&1 1>&2 2>&3 || true
  else
    local s; read -rsp "$(printf '%b' "$2"): " s; printf '\n%s\n' "$s"
  fi
}

ui_caixa_progresso() { # $1=titulo; linhas no stdin
  if _ui_dialog; then dialog --title "$1" --progressbox 20 78
  else tee -a "$UI_LOG"; fi
}
