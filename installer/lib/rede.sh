#!/usr/bin/env bash
# rede.sh — WiFi/Ethernet via NetworkManager (nmcli).

rede_disponivel() { command -v nmcli >/dev/null 2>&1; }

rede_ativar() { nmcli radio wifi on 2>/dev/null || true; nmcli networking on || true; sleep 2; }

rede_conectada() { [ "$(nmcli -t -f STATE general 2>/dev/null)" = "connected" ]; }

rede_aguardar() { # até 30s
  local i
  for i in $(seq 1 15); do rede_conectada && return 0; sleep 2; done
  return 1
}

rede_varrer() { # "SSID|SINAL|SEGURANCA", únicos, sem vazio
  nmcli -t -f SSID,SIGNAL,SECURITY device wifi list --rescan yes 2>/dev/null \
    | awk -F':' 'NF >= 2 && $1 != "" && !seen[$1]++'
}

rede_conectar() { # $1=ssid $2=senha
  nmcli device wifi connect "$1" password "$2" >/dev/null 2>&1
}

rede_fluxo() { # 0=conectado 1=sem rede/pulado (loop até 3 senhas)
  rede_disponivel || { ui_erro "Rede" "NetworkManager indisponível."; return 1; }
  rede_ativar
  local pares=() l ssid senha tent=0
  while IFS= read -r l; do
    pares+=("$(printf '%s' "$l" | cut -d: -f1)|$(printf '%s' "$l" | sed 's/|/ /g')")
  done < <(rede_varrer)
  if [ "${#pares[@]}" -eq 0 ]; then
    ui_msg "Rede" "Nenhuma rede WiFi encontrada.\nVocê pode instalar SEM rede (tudo offline)."
    return 1
  fi
  while [ "$tent" -lt 3 ]; do
    ssid=$(ui_menu "Rede WiFi" "Escolha a rede (ou cancele para pular):" "${pares[@]}") || ssid=""
    [ -n "$ssid" ] || return 1
    senha=$(ui_senha "Senha de $ssid" "Digite a senha:")
    if rede_conectar "$ssid" "$senha" && rede_aguardar; then
      ui_msg "Rede" "Conectado a $ssid!"
      return 0
    fi
    tent=$((tent + 1))
    ui_erro "Conexão" "Falha ao conectar em $ssid (tentativa $tent/3)."
  done
  return 1
}

rede_copiar_perfis() { # perfis do live → destino instalado (perm 600)
  if compgen -G "/etc/NetworkManager/system-connections/*" >/dev/null 2>&1; then
    install -d -m 700 /mnt/etc/NetworkManager/system-connections
    cp -a /etc/NetworkManager/system-connections/. /mnt/etc/NetworkManager/system-connections/
    ui_log "PERFIS-NM-COPIADOS"
  fi
}
