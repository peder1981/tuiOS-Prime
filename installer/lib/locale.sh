#!/usr/bin/env bash
# locale.sh — padrões brasileiros do sistema instalado.

LOCALE_PADRAO="pt_BR.UTF-8"
TECLADO_PADRAO="br-abnt2"
FUZO_PADRAO="America/Sao_Paulo"

locale_resumo() {
  printf 'locale %s · teclado %s · fuso %s' "$LOCALE_PADRAO" "$TECLADO_PADRAO" "$FUZO_PADRAO"
}

locale_aplicar() { # $1=configuration.nix — garante o bloco BR se faltar
  local cfg="$1"
  grep -q "time.timeZone" "$cfg" 2>/dev/null && return 0
  cat >> "$cfg" <<EOC

  time.timeZone = "$FUZO_PADRAO";
  i18n.defaultLocale = "$LOCALE_PADRAO";
  console.keyMap = "$TECLADO_PADRAO";
EOC
}
