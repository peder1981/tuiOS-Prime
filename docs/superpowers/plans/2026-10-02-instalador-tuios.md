# Instalador tuiOS-Prime (tuios-instalar) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Embarcar um instalador PT-BR (`tuios-instalar`) na ISO NixOS do tuiOS-Prime que conecta WiFi, particiona o disco escolhido e persiste o sistema (tuiOS + AdvPP + locale BR) com boot UEFI.

**Architecture:** Script bash+dialog dividido em `installer/lib/*` (UI, disco, rede, locale, instalação), empacotado como derivação Nix e embarcado na ISO. A instalação usa o NixOS nativo (`nixos-generate-config` + `nixos-install` com `NIX_PATH` apontando para o nixpkgs já presente na store do pendrive — 100% offline). O módulo de sessão tuiOS foi fatorado de `iso.nix` para `tuios-session.nix` e é compartilhado entre live e instalado.

**Tech Stack:** Bash 4 + dialog, nmcli (NetworkManager), parted/mkfs, nixos-install, Nix flakes, QEMU+OVMF (teste), GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-02-instalador-tuios-design.md` (aprovado)

## Global Constraints

- Idioma PT-BR em toda UI, docs, mensagens e commits (identificadores técnicos em inglês).
- Instalação DEVE funcionar offline (closure na store do pendrive; `NIX_PATH=nixpkgs=<path da ISO>`).
- Confirmação destrutiva: literal `APAGAR` digitado; modo auto exige `--aceitar-tudo`.
- Dispositivos do live (raiz/`/iso`/partições montadas) NUNCA aparecem como candidatos.
- Padrões do sistema instalado: locale `pt_BR.UTF-8`, teclado `br-abnt2`, fuso `America/Sao_Paulo`, autologin root + sessão tuiOS, NetworkManager.
- Partições: p1 EFI 512MiB FAT32, p2 swap 2048MiB, p3 root ext4 (resto), GPT.
- Tokens de assert: `INSTAL-OK` (instalação), `INSTAL-TEST-OK` (script), `active` (tuios-session pós-install).
- `--impure` continua obrigatório no build da ISO (advplc usa binário local; ver Task 11/decisão de vendorização).
- Commits sem qualquer atribuição de assistente (Lei 2, Anexo D).
- **Nix desta máquina:** o daemon (`nix` puro) nega o socket. Usar store local — prefixar TODOS os comandos `nix` com `nix --store /tmp/nix-official`. O `--out-link result` gera symlink para `/nix/store/...` (inexistente); caminho físico: ler `readlink` e trocar o prefixo `/nix/store` → `/tmp/nix-official/nix/store` (helper: `phys(){ readlink "$1" | sed 's#/nix/store#/tmp/nix-official/nix/store#'; }`).

---

### Task 1: Módulo compartilhado `nixos/tuios-session.nix`

**Files:**
- Create: `nixos/tuios-session.nix`
- Modify: `nixos/iso.nix` (remover bloco inline de sessão/autologin, importar módulo)

**Interfaces:**
- Consumes: `pkgs`, `lib` (module args padrão).
- Produz: módulo NixOS auto-contido (autologin root, getty@tty1 off, serviço `tuios-session`, `console=ttyS0`) — importado por `iso.nix` (Task 1) e pelo template instalado (Task 2).

- [ ] **Step 1: Criar `nixos/tuios-session.nix`**

```nix
{ pkgs, lib, ... }:
# Sessão tuiOS + autologin root — compartilhado entre a ISO (live) e o
# sistema instalado. tty1 exclusivo da sessão (getty disputava e causava
# SIGHUP — ver commit c98bd9b).
{
  boot.kernelParams = [ "console=ttyS0,115200n8" ];

  services.getty.autologinUser = lib.mkForce "root";
  systemd.services."getty@tty1".enable = false;
  systemd.services."autovt@tty1".enable = false;

  systemd.services.tuios-session = {
    description = "Sessão tuiOS em tela cheia";
    after = [ "getty@tty1.service" ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      ExecStart = "${pkgs.bash}/bin/bash -lc 'exec tuios attach || exec tuios'";
      StandardInput = "tty";
      TTYPath = "/dev/tty1";
      TTYReset = true;
      TTYVHangup = true;
    };
  };
}
```

- [ ] **Step 2: Refatorar `iso.nix` para importar o módulo**

Remover de `nixos/iso.nix` as linhas do comentário "Login automático..." até o fim do bloco `systemd.services.tuios-session` (linhas 11–32) e adicionar o import:

```nix
  imports = [
    (modulesPath + "/installer/cd-dvd/installation-cd-minimal.nix")
    ./hardware-qemu.nix
    ./tuios-session.nix
  ];
```

- [ ] **Step 3: Avaliar que o sistema live continua consistente**

Run: `cd /home/peder/Projetos/tuiOS-Prime && nix eval --impure .#nixosConfigurations.iso.config.system.build.toplevel.drvPath`
Expected: imprime um caminho `/nix/store/...drv` (sem error).

- [ ] **Step 4: Commit**

```bash
git add nixos/tuios-session.nix nixos/iso.nix
git commit -m "[REF] — Extrair sessão tuiOS para módulo compartilhado tuios-session.nix"
```

---

### Task 2: Template do sistema instalado + material embutido na ISO

**Files:**
- Create: `nixos/instalado/configuration.nix`
- Modify: `nixos/iso.nix` (adicionar `environment.etc."tuios-installer/..."` + arg `nixpkgsPath`)
- Modify: `flake.nix` (`specialArgs` ganha `nixpkgsPath`)

**Interfaces:**
- Consumes: `tuios` e `advplc` (specialArgs existentes), `./tuios-session.nix` (Task 1), `nixpkgs.outPath` (flake).
- Produz (arquivos no live em `/etc/tuios-installer/`): `configuration.nix`, `tuios-session.nix`, `tuios-env.nix` (store paths de tuios/advplc via `builtins.storePath`), `nixpkgs-path` (path do nixpkgs). Task 6 consome todos com `install -m644` para `/mnt/etc/nixos/`.

- [ ] **Step 1: Criar `nixos/instalado/configuration.nix`**

```nix
# Configuração do sistema tuiOS-Prime INSTALADO em disco.
# Gerada pelo tuios-instalar; pós-instalação: nixos-rebuild switch.
{ config, pkgs, lib, ... }:
let
  env = import ./tuios-env.nix;
in {
  imports = [
    ./hardware-configuration.nix
    ./tuios-session.nix
  ];

  networking.hostName = "tuios-prime";
  networking.networkmanager.enable = true;

  time.timeZone = "America/Sao_Paulo";
  i18n.defaultLocale = "pt_BR.UTF-8";
  console.keyMap = "br-abnt2";

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = false;

  environment.systemPackages = [
    env.tuios
    env.advplc
    pkgs.htop
    pkgs.pciutils
    pkgs.usbutils
    pkgs.vim
    pkgs.git
    pkgs.go
    pkgs.dialog
    pkgs.networkmanager
  ];

  environment.variables = { ADVPP_DB = "/var/lib/advpp/advpp.db"; };
  systemd.tmpfiles.rules = [ "d /var/lib/advpp 0755 root root -" ];

  system.stateVersion = "24.05";
}
```

- [ ] **Step 2: `flake.nix` — passar `nixpkgsPath`**

```nix
specialArgs = { inherit tuios advplc; nixpkgsPath = nixpkgs.outPath; };
```

- [ ] **Step 3: `iso.nix` — embutir material do instalador (adicionar ao módulo)**

```nix
  # NetworkManager: fluxo de WiFi do instalador (nmcli) + pós-instalação
  networking.networkmanager.enable = true;

  # Material do instalador embutido na ISO (/etc/tuios-installer/)
  environment.etc."tuios-installer/configuration.nix".source = ./instalado/configuration.nix;
  environment.etc."tuios-installer/tuios-session.nix".source = ./tuios-session.nix;
  environment.etc."tuios-installer/tuios-env.nix".text = ''
    {
      tuios  = builtins.storePath ${tuios.packages.${pkgs.system}.default};
      advplc = builtins.storePath ${advplc};
    }
  '';
  environment.etc."tuios-installer/nixpkgs-path".text = nixpkgsPath;
```

- [ ] **Step 4: Avaliar**

Run: `nix eval --impure .#nixosConfigurations.iso.config.system.build.toplevel.drvPath`
Expected: `/nix/store/...drv` sem erros.
Run: `nix eval --impure --json .#nixosConfigurations.iso.config.environment.etc --apply 'x: builtins.attrNames x' | grep tuios-installer`
Expected: contém `tuios-installer/configuration.nix`, `tuios-installer/tuios-env.nix`, `tuios-installer/nixpkgs-path`, `tuios-installer/tuios-session.nix`.

- [ ] **Step 5: Commit**

```bash
git add nixos/instalado/configuration.nix nixos/iso.nix flake.nix
git commit -m "[FEAT] — Template do sistema instalado + etc tuios-installer embarcado na ISO"
```

---

### Task 3: `installer/lib/ui.sh` — caixas do assistente

**Files:**
- Create: `installer/lib/ui.sh`
- Create: `installer/tuios-instalar` (stub de teste mínimo nesta task — entry point real na Task 7)

**Interfaces:**
- Produz (funções consumidas por todas as demais libs e pelo entry): `UI` (1=dialog,0=texto), `ui_log`, `ui_msg titulo texto`, `ui_erro titulo texto`, `ui_erro_rc`, `ui_yesno titulo texto` (0=sim), `ui_confirma_texto titulo texto literal` (0=confirmou), `ui_menu titulo texto tag|desc ...` (echo da tag), `ui_senha titulo texto` (echo da senha), `ui_caixa_progresso titulo` (stdin→caixa/tee).

- [ ] **Step 1: Escrever o teste de sintaxe (falha primeiro)**

```bash
mkdir -p installer/lib
printf '#!/usr/bin/env bash\nset -euo pipefail\n. "$(dirname "$0")/lib/ui.sh"\n[ "${UI:-}" = "1" ] && echo ui-contrato-ok\n' > installer/tuios-instalar
bash -n installer/tuios-instalar && echo SINTAXE-OK
touch installer/lib/ui.sh && bash -n installer/lib/ui.sh
```

Run: `bash installer/tuios-instalar`
Expected: falha com erro (ui.sh vazio não define nada — `UI` não existe com `set -u`) → contrato quebrado = teste vermelho.

- [ ] **Step 2: Implementar `installer/lib/ui.sh`**

```bash
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
```

- [ ] **Step 3: Rodar o teste de contrato**

Run: `bash installer/tuios-instalar`
Expected: `ui-contrato-ok` e exit 0.

- [ ] **Step 4: `bash -n` nas duas + smoke do modo texto**

```bash
bash -n installer/lib/ui.sh && bash -n installer/tuios-instalar && echo SINTAXE-OK
UI=0 bash -c '. installer/lib/ui.sh; ui_msg "Titulo" "linha1\nlinha2"; ui_menu "M" "escolha" "a|Alpha" "b|Beta"'
```
Expected: `SINTAXE-OK`; imprime "== Titulo ==" com 2 linhas e o menu com as 2 opções (entrada EOF ⇒ tag vazia, aceitável).

- [ ] **Step 5: Commit**

```bash
git add installer/
git commit -m "[FEAT] — ui.sh: caixas dialog/texto do assistente tuios-instalar"
```

---

### Task 4: `installer/lib/disco.sh` — detecção segura + particionamento

**Files:**
- Create: `installer/lib/disco.sh`

**Interfaces:**
- Consumes: `ui_menu`, `ui_confirma_texto`, `ui_erro` (Task 3).
- Produz: `discos_candidatos` (linhas `tag|descrição`, um disco por linha), `disco_e_live d` (0=o disco hospeda o live), `particionar d` → define `P1/P2/P3` e formata, `nomes_particoes d` → define `P1/P2/P3` sem formatar.

- [ ] **Step 1: Teste de sintaxe (falha primeiro)**

```bash
touch installer/lib/disco.sh && bash -n installer/lib/disco.sh
bash -c '. installer/lib/disco.sh; type particionar' 2>&1 | grep -q "não definida\|not found" && echo TESTE-VERMELHO-OK
```

- [ ] **Step 2: Implementar `installer/lib/disco.sh`**

```bash
#!/usr/bin/env bash
# disco.sh — candidatos seguros a disco + particionamento GPT/UEFI.

# Discos que hospedam o live (raiz, /iso, ou com partição montada)
_discos_excluidos() {
  local mnt src d
  for mnt in / /iso /boot; do
    src=$(findmnt -n -o SOURCE "$mnt" 2>/dev/null || true)
    src=${src%%[*}                       # tira "[root]" de overlay
    [ -b "$src" ] || continue
    d=$(lsblk -no PKNAME "$src" 2>/dev/null | head -1 || true)
    if [ -n "$d" ]; then printf '/dev/%s\n' "$d"; else printf '%s\n' "$src"; fi
  done | sort -u
}

disco_e_live() { # $1=/dev/xxx
  _discos_excluidos | grep -qx "$1"
}

discos_candidatos() { # linhas "tag|descrição"; tag = /dev/xxx
  local excl nome tam trans modelo e
  excl=$(_discos_excluidos)
  while read -r nome tam trans modelo; do
    case "$nome" in *zram*|*loop*) continue;; esac
    if printf '%s\n' "$excl" | grep -qx "$nome"; then continue; fi
    if lsblk -nrpo MOUNTPOINT "$nome" 2>/dev/null | grep -q '^/'; then continue; fi
    printf '%s|%s %s %s %s\n' "$nome" "$tam" "$trans" "$modelo"
  done < <(lsblk -dpno NAME,SIZE,TRAN,MODEL 2>/dev/null)
}

nomes_particoes() { # $1=/dev/sda|nvme0n1|mmcblk0 → P1 P2 P3
  local s="$1"
  if [[ "$1" =~ (nvme|mmcblk|loop)[0-9]+$ ]]; then s="${1}p"; fi
  P1="${s}1"; P2="${s}2"; P3="${s}3"
}

particionar() { # $1=/dev/xxx — APAGA o disco e cria EFI/swap/root
  local d="$1"
  ui_log "PARTICIONAR $d"
  wipefs -af "$d" || true
  parted -s "$d" mklabel gpt
  parted -s "$d" mkpart primary fat32 1MiB 513MiB
  parted -s "$d" set 1 esp on
  parted -s "$d" mkpart primary linux-swap 513MiB 2561MiB
  parted -s "$d" mkpart primary ext4 2561MiB 100%
  partprobe "$d" || true
  sleep 1
  nomes_particoes "$d"
  mkfs.fat -F32 -n EFI "$P1"
  mkswap -L swap "$P2"
  mkfs.ext4 -F -L tuios "$P3"
  ui_log "PARTICIONADO $d → $P1 $P2 $P3"
}
```

- [ ] **Step 3: Testar detecção no host (somente leitura)**

```bash
bash -c '. installer/lib/disco.sh; echo "-- candidatos --"; discos_candidatos; echo "-- excluidos --"; _discos_excluidos'
```
Expected: a lista de candidatos NÃO contém o disco do sistema atual (`nvme0n1`/`sda` montados); excluidos mostra o disco da raiz. (`/iso` inexistente no host é ignorado com `|| true`.)

- [ ] **Step 4: `bash -n` + commit**

```bash
bash -n installer/lib/disco.sh && echo SINTAXE-OK
git add installer/lib/disco.sh
git commit -m "[FEAT] — disco.sh: candidatos seguros e particionamento GPT/UEFI"
```

---

### Task 5: `installer/lib/rede.sh` + `installer/lib/locale.sh`

**Files:**
- Create: `installer/lib/rede.sh`
- Create: `installer/lib/locale.sh`

**Interfaces:**
- Consumes: `ui_menu`, `ui_senha`, `ui_msg`, `ui_erro` (Task 3).
- Produz: `rede_fluxo` (0=conectado, 1=sem rede/pulado; usa `NM` na UI), `rede_copiar_perfis` (copia perfis para `/mnt/etc/...`), `rede_conectada` (0=sim); `locale_aplicar arquivo` (garante locale BR no configuration.nix), constantes `LOCALE_PADRAO/TECLADO_PADRAO/FUZO_PADRAO`, `locale_resumo` (texto para a etapa de resumo). Task 6 consome tudo.

- [ ] **Step 1: Teste de sintaxe (falha primeiro)**

```bash
touch installer/lib/rede.sh installer/lib/locale.sh
bash -n installer/lib/rede.sh && bash -n installer/lib/locale.sh
bash -c '. installer/lib/rede.sh; type rede_fluxo' 2>&1 | grep -q "not found\|não definida" && echo TESTE-VERMELHO-OK
```

- [ ] **Step 2: Implementar `installer/lib/rede.sh`**

```bash
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
    ssid=$(ui_menu "Rede WiFi" "Escolha a rede (ou deixe vazio para pular):" "${pares[@]}") || ssid=""
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
```

- [ ] **Step 3: Implementar `installer/lib/locale.sh`**

```bash
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
  cat >> "$cfg" <<EOF

  time.timeZone = "$FUZO_PADRAO";
  i18n.defaultLocale = "$LOCALE_PADRAO";
  console.keyMap = "$TECLADO_PADRAO";
EOF
}
```

- [ ] **Step 4: Testes**

```bash
bash -n installer/lib/rede.sh && bash -n installer/lib/locale.sh && echo SINTAXE-OK
bash -c '. installer/lib/locale.sh; f=$(mktemp); echo "{" > $f; locale_aplicar $f; grep -c time.timeZone $f; rm -f $f'
```
Expected: `SINTAXE-OK` e `1` (bloco BR aplicado).

- [ ] **Step 5: Commit**

```bash
git add installer/lib/
git commit -m "[FEAT] — rede.sh (nmcli) + locale.sh (padrões BR) do assistente"
```

---

### Task 6: `installer/lib/instalar.sh` — montagem → config → nixos-install → boot

**Files:**
- Create: `installer/lib/instalar.sh`

**Interfaces:**
- Consumes: `nomes_particoes`, `particionar` (Task 4), `rede_copiar_perfis` (Task 5), `locale_aplicar` (Task 5), `ui_log` (Task 3).
- Produz: `instalar_executar $disco` (chama `particionar` se `PARTICIONAR=1`; imprime etapas + `INSTAL-OK` no fim; rc≠0 em falha), `limpar_montagens` (para `trap EXIT`), variáveis `P1/P2/P3/DISCO_ALVO`.

- [ ] **Step 1: Teste de sintaxe (falha primeiro)**

```bash
touch installer/lib/instalar.sh && bash -n installer/lib/instalar.sh
bash -c '. installer/lib/instalar.sh; type instalar_executar' 2>&1 | grep -q "not found\|não definida" && echo TESTE-VERMELHO-OK
```

- [ ] **Step 2: Implementar `installer/lib/instalar.sh`**

```bash
#!/usr/bin/env bash
# instalar.sh — pipeline de instalação em disco (offline-first).

ETAPA_N=0
PARTICIONAR="${PARTICIONAR:-1}"   # 0 = disco já particionado (retomada)

_etapa() { ETAPA_N=$((ETAPA_N + 1)); printf -- '--- Etapa %s/5: %s ---\n' "$ETAPA_N" "$*"; ui_log "ETAPA $ETAPA_N: $*"; }

limpar_montagens() { # idempotente, para trap EXIT
  set +e
  [ -n "${P2:-}" ] && swapoff "$P2" 2>/dev/null
  umount -R /mnt 2>/dev/null
  set -e
}

montar_destino() {
  _etapa "Montando discos ($P3 → /mnt, $P1 → /mnt/boot, swap $P2)"
  mount "$P3" /mnt
  mkdir -p /mnt/boot
  mount "$P1" /mnt/boot
  swapon "$P2"
}

gerar_config() {
  _etapa "Configurando sistema (hardware, locale BR, sessão tuiOS, rede)"
  nixos-generate-config --root /mnt
  install -m 644 /etc/tuios-installer/configuration.nix /mnt/etc/nixos/configuration.nix
  install -m 644 /etc/tuios-installer/tuios-session.nix /mnt/etc/nixos/tuios-session.nix
  install -m 644 /etc/tuios-installer/tuios-env.nix     /mnt/etc/nixos/tuios-env.nix
  locale_aplicar /mnt/etc/nixos/configuration.nix
  if [ ! -d /sys/firmware/efi ]; then
    # BIOS: GRUB no disco, desliga systemd-boot (mkForce p/ vencer o template)
    cat >> /mnt/etc/nixos/configuration.nix <<EOF

  boot.loader.systemd-boot.enable = lib.mkForce false;
  boot.loader.grub.enable = true;
  boot.loader.grub.device = "$DISCO_ALVO";
EOF
  fi
  rede_copiar_perfis
}

instalar_sistema() {
  _etapa "Instalando tuiOS-Prime (offline — closure na store do pendrive)"
  export NIX_PATH="nixpkgs=$(cat /etc/tuios-installer/nixpkgs-path)"
  # --no-channel-copy: não tenta copiar canal (sem rede)
  nixos-install --root /mnt --no-root-passwd --no-channel-copy
}

instalar_bootloader() {
  _etapa "Verificando bootloader"
  if [ -d /sys/firmware/efi ]; then
    if [ ! -d /mnt/boot/EFI ]; then
      echo "AVISO: bootctl via ativação não gravou EFI — rodando bootctl manual"
      bootctl install --esp-path=/mnt/boot
    fi
    echo "Bootloader: systemd-boot (UEFI)"
  else
    echo "Bootloader: GRUB (BIOS)"
  fi
}

instalar_executar() { # $1=/dev/alvo (DISCO já escolhido/validado)
  set -e
  DISCO_ALVO="$1"
  if [ "$PARTICIONAR" = 1 ]; then
    _etapa "Particionando $DISCO_ALVO (GPT: EFI 512M · swap 2G · root ext4)"
    particionar "$DISCO_ALVO"
  else
    nomes_particoes "$DISCO_ALVO"
    _etapa "Reutilizando partições existentes de $DISCO_ALVO"
  fi
  trap limpar_montagens EXIT
  montar_destino
  gerar_config
  instalar_sistema
  instalar_bootloader
  sync
  trap - EXIT
  printf '\nINSTAL-OK\n'
}
```

- [ ] **Step 3: Teste de contrato + sintaxe**

```bash
bash -n installer/lib/instalar.sh && echo SINTAXE-OK
bash -c 'set -euo pipefail; . installer/lib/ui.sh; . installer/lib/instalar.sh; type instalar_executar >/dev/null && echo CONTRATO-OK'
```
Expected: `SINTAXE-OK` e `CONTRATO-OK`.

- [ ] **Step 4: Validar flags do `nixos-install` (regra de validação de símbolos)**

Run: `nixos-install --help 2>&1 | grep -E "no-root-passwd|no-channel-copy" || man nixos-install 2>/dev/null | grep -E "no-root-passwd|no-channel-copy" || echo "VALIDAR-NO-LIVE"`
Expected: as duas flags documentadas (se `VALIDAR-NO-LIVE`, conferir dentro da ISO na Task 9 e ajustar `instalar_sistema`).

- [ ] **Step 5: Commit**

```bash
git add installer/lib/instalar.sh
git commit -m "[FEAT] — instalar.sh: pipeline mount→config→nixos-install→boot (offline)"
```

---

### Task 7: Entry point `installer/tuios-instalar` (wizard 5 etapas + modo auto)

**Files:**
- Modify: `installer/tuios-instalar` (substitui o stub da Task 3)

**Interfaces:**
- Consumes: todas as libs (Tasks 3–6).
- Produz CLI: `tuios-instalar` (interativo) | `tuios-instalar --auto --disco /dev/vdX --sem-rede --aceitar-tudo` (CI) | `--help` | `--version`.

- [ ] **Step 1: Implementar o entry point completo**

```bash
#!/usr/bin/env bash
# tuios-instalar — assistente de instalação do tuiOS-Prime (PT-BR)
set -euo pipefail

VERS="1.0.0"
BASE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB="$BASE/lib"
[ -d "$LIB" ] || LIB="/etc/tuios-installer/lib"

. "$LIB/ui.sh"
. "$LIB/disco.sh"
. "$LIB/rede.sh"
. "$LIB/locale.sh"
. "$LIB/instalar.sh"

AUTO=0; SEM_REDE=0; ACEITAR=0; DISCO_ESCOLHIDO=""

while [ $# -gt 0 ]; do
  case "$1" in
    --auto)          AUTO=1; UI=0; shift ;;
    --disco)         DISCO_ESCOLHIDO="${2:?uso: --disco /dev/x}"; shift 2 ;;
    --sem-rede)      SEM_REDE=1; shift ;;
    --aceitar-tudo)  ACEITAR=1; shift ;;
    -h|--help) sed -n '2,14p' "$0"; exit 0 ;;
    --version) echo "tuios-instalar $VERS"; exit 0 ;;
    *) echo "opção desconhecida: $1 (use --help)" >&2; exit 2 ;;
  esac
done

[ "$(id -u)" = 0 ] || { echo "Execute como root: sudo tuios-instalar" >&2; exit 1; }
touch "$UI_LOG" 2>/dev/null || UI_LOG=/tmp/tuios-install.log

# ---------- Etapa 1: pré-requisitos e disco ----------
etapa1() {
  ui_msg "tuiOS-Prime — Instalador v$VERS" \
    "Este assistente instala o tuiOS-Prime em disco.\n\nO disco escolhido sera APAGADO por completo.\n%s" "$(locale_resumo)"
  if [ ! -d /sys/firmware/efi ]; then
    ui_erro "UEFI" "Modo UEFI não detectado — a instalação exige UEFI (Chromebook/PC moderno)."
    return 1
  fi
  if [ -z "$DISCO_ESCOLHIDO" ]; then
    local candidatos=() p
    while IFS= read -r p; do [ -n "$p" ] && candidatos+=("$p"); done < <(discos_candidatos)
    if [ "${#candidatos[@]}" -eq 0 ]; then
      ui_erro "Disco" "Nenhum disco utilizável encontrado."
      return 1
    fi
    DISCO_ESCOLHIDO=$(ui_menu "Escolher disco" "⚠ O disco selecionado sera APAGADO." "${candidatos[@]}") || DISCO_ESCOLHIDO=""
    [ -n "$DISCO_ESCOLHIDO" ] || { ui_msg "Cancelado" "Nenhum disco selecionado."; return 1; }
  fi
  [ -b "$DISCO_ESCOLHIDO" ] || { ui_erro "Disco" "Disco inexistente: $DISCO_ESCOLHIDO"; return 1; }
  if disco_e_live "$DISCO_ESCOLHIDO"; then
    ui_erro "Disco" "$DISCO_ESCOLHIDO hospeda o sistema ao vivo — escolha outro disco."
    return 1
  fi
  if [ "$ACEITAR" != 1 ]; then
    ui_confirma_texto "Confirmação final" \
      "Disco: $DISCO_ESCOLHIDO\nTODOS OS DADOS SERAO PERDIDOS." "APAGAR" \
      || { ui_msg "Cancelado" "Instalação cancelada pelo usuário."; exit 0; }
  fi
  ui_log "DISCO-ALVO $DISCO_ESCOLHIDO"
}

# ---------- Etapa 2: rede ----------
etapa2() {
  if [ "$SEM_REDE" = 1 ]; then ui_msg "Rede" "Modo --sem-rede: instalação offline."; return 0; fi
  rede_fluxo || ui_msg "Rede" "Seguindo SEM rede — a instalação funciona offline.\nO WiFi pode ser configurado depois, no sistema instalado."
}

# ---------- Etapa 3: resumo ----------
etapa3() {
  local rede="não conectada"
  rede_conectada && rede="conectada"
  local boot="systemd-boot (UEFI)"; [ -d /sys/firmware/efi ] || boot="GRUB (BIOS)"
  if [ "$ACEITAR" != 1 ]; then
    ui_yesno "Resumo da instalação" \
      "Disco: $DISCO_ESCOLHIDO\nPartições: EFI 512M · swap 2G · root ext4 (resto)\nRede: $rede\nBoot: $boot\n%s\n\nProsseguir?" \
      || { ui_msg "Cancelado" "Instalação cancelada."; exit 0; }
  fi
}

# ---------- Etapa 4/5: instalar ----------
etapa4() {
  if instalar_executar "$DISCO_ESCOLHIDO" 2>&1 | ui_caixa_progresso "Instalando tuiOS-Prime"; then
    return 0
  fi
  return 1
}

main() {
  etapa1 || exit 1
  etapa2
  etapa3
  if etapa4; then
    ui_msg "Concluído ✅" \
      "Instalação concluída com sucesso!\n\nRemova o pendrive e reinicie.\nNo próximo boot o tuiOS inicia automaticamente."
    exit 0
  fi
  ui_erro "Falha na instalação" \
    "A instalação falhou. Detalhes em:\n$UI_LOG\n\nÚltimas linhas:\n$(tail -n 12 "$UI_LOG" 2>/dev/null || true)"
  exit 1
}

main
```

- [ ] **Step 2: Sintaxe + CLI**

```bash
bash -n installer/tuios-instalar && echo SINTAXE-OK
bash installer/tuios-instalar --version
bash installer/tuios-instalar --help | head -3
bash installer/tuios-instalar --opcao-invalida 2>&1; echo "rc=$?"
```
Expected: `SINTAXE-OK`; `tuios-instalar 1.0.0`; ajuda; `opção desconhecida` com `rc=2`.

- [ ] **Step 3: Modo auto sem root (falha com mensagem correta)**

```bash
bash installer/tuios-instalar --auto --disco /dev/null --sem-rede --aceitar-tudo 2>&1; echo "rc=$?"
```
Expected (se rodando sem root): mensagem "Execute como root" e `rc=1`.

- [ ] **Step 4: Commit**

```bash
git add installer/tuios-instalar
git commit -m "[FEAT] — tuios-instalar: wizard 5 etapas + modo --auto para CI"
```

---

### Task 8: Empacotar na ISO (`installer/package.nix` + wiring em `iso.nix`)

**Files:**
- Create: `installer/package.nix`
- Modify: `nixos/iso.nix` (systemPackages += tuios-instalar)
- Modify: `nixos/instalado/configuration.nix` (systemPackages += tuios-instalar)

**Interfaces:**
- Consumes: `installer/` (Tasks 3–7), derivations do nixpkgs.
- Produz: pacote `tuios-instalar` (`$out/bin/tuios-instalar` com libs em `$out/lib` e PATH prefixado com dialog/nmcli/parted/...); o entry resolve `LIB=$BASE/lib` automaticamente.

- [ ] **Step 1: Criar `installer/package.nix`**

```nix
{ lib, stdenv, makeWrapper
, dialog, networkmanager, parted, dosfstools, e2fsprogs, gptfdisk
, util-linux, coreutils, findutils, gawk, gnugrep, systemd }:

stdenv.mkDerivation {
  pname = "tuios-instalar";
  version = "1.0.0";
  src = ./.;

  dontConfigure = true;
  dontBuild = true;

  nativeBuildInputs = [ makeWrapper ];

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin $out/lib
    cp tuios-instalar $out/bin/
    cp -r lib/. $out/lib/
    chmod +x $out/bin/tuios-instalar
    runHook postInstall
  '';

  postFixup = ''
    wrapProgram $out/bin/tuios-instalar --prefix PATH : ${
      lib.makeBinPath [ dialog networkmanager parted dosfstools e2fsprogs
                        gptfdisk util-linux coreutils findutils gawk gnugrep systemd ]
    }
  '';
}
```

- [ ] **Step 2: Wiring na ISO — adicionar em `nixos/iso.nix` (systemPackages)**

```nix
    # Assistente de instalação
    (pkgs.callPackage ../installer/package.nix { })
```

- [ ] **Step 3: Mesmo pacote no sistema instalado — adicionar em `nixos/instalado/configuration.nix`**

O template não tem `../installer` no store em tempo de eval do destino… usar o mesmo caminho relativo funciona porque o template é copiado para `/mnt/etc/nixos/` e o eval usa o repositório? **NÃO** — o eval é a partir de `/mnt/etc/nixos` sem o repo. Solução: o template referencia o pacote pelo **store path embutido**:

Em `nixos/iso.nix` (junto dos demais embeds):

```nix
  environment.etc."tuios-installer/pkg-path".text =
    "${pkgs.callPackage ../installer/package.nix { }}";
```

E no template `nixos/instalado/configuration.nix`, trocar a lista para incluir:

```nix
    (builtins.storePath (builtins.readFile "/etc/tuios-installer/pkg-path"))
```

> Nota: `builtins.readFile` + `storePath` mantém o closure via contexto? **Não** (readFile perde contexto). Correção usada: gerar no iso.nix um snippet Nix com o path literal:

`environment.etc."tuios-installer/tuios-installer-pkg.nix".text = "{ pkg = builtins.storePath ${pkgs.callPackage ../installer/package.nix { }}; }";`

e no template: `let env = import ./tuios-env.nix; installerPkg = import ./tuios-installer-pkg.nix; in ... systemPackages = [ env.tuios env.advplc installerPkg.pkg ... ]`. O `storePath` preserva contexto ⇒ o pacote entra no closure alvo durante o `nixos-install` (que roda no live, onde o path existe). ✔

- [ ] **Step 4: Avaliar**

Run: `nix eval --impure .#nixosConfigurations.iso.config.system.build.toplevel.drvPath`
Expected: `/nix/store/...drv` sem erro.
Run: `nix build --impure .#packages.x86_64-linux.tuios-instalar`? (não existe como output do flake — avaliar via:`nix eval --impure --raw '.#nixosConfigurations.iso.config.environment.systemPackages' --apply 'ps: builtins.concatStringsSep "\n" (map (p: p.name or "?") ps)' | grep tuios-instalar`)
Expected: linha `tuios-instalar-1.0.0`.

- [ ] **Step 5: Commit**

```bash
git add installer/package.nix nixos/iso.nix nixos/instalado/configuration.nix
git commit -m "[FEAT] — tuios-instalar empacotado na ISO e no sistema instalado"
```

---

### Task 9: Build da ISO + smoke test no live (material embarcado)

**Files:**
- Modify: `Justfile` (target `iso` ganha `--out-link result` já existe em `qemu-iso` — manter)

**Interfaces:**
- Produz: ISO com `/etc/tuios-installer/*` + binário `tuios-instalar` no PATH; validação das flags do `nixos-install` dentro do live (passo pendente da Task 6).

- [ ] **Step 1: Build**

Run: `cd /home/peder/Projetos/tuiOS-Prime && nix build --impure .#nixosConfigurations.iso.config.system.build.isoImage --print-out-paths --out-link result 2>&1 | tail -3`
Expected: imprime `/nix/store/...-nixos-...iso` (sem `builder failed`).

- [ ] **Step 2: Extrair kernel/initrd e bootar no QEMU (serial)**

```bash
ISO=$(readlink -f result/iso/*.iso || ls result/iso/*.iso)
mkdir -p /tmp/smoke-iso && rm -rf /tmp/smoke-iso/*
bsdtar -xf "$ISO" -C /tmp/smoke-iso boot/bzImage boot/initrd
SYS=$(nix eval --impure --raw .#nixosConfigurations.iso.config.system.build.toplevel 2>/dev/null | grep '^/nix/store' | tail -1)
[ -n "$SYS" ] || SYS=$(nix eval --impure --raw .#nixosConfigurations.iso.config.system.build.toplevel)
rm -f /tmp/sm-in; mkfifo /tmp/sm-in; sleep 480 > /tmp/sm-in &
timeout 460 qemu-system-x86_64 -M q35 -m 2048M -display none -serial stdio \
  -kernel /tmp/smoke-iso/boot/bzImage -initrd /tmp/smoke-iso/boot/initrd \
  -append "init=$SYS/init root=LABEL=nixos-minimal-24.05-x86_64 boot.shell_on_fail nohibernate loglevel=4 console=ttyS0,115200n8" \
  -cdrom "$ISO" < /tmp/sm-in > /tmp/smoke-iso/log 2>&1 || true
```
(Aguardar `root@tuios-prime:~#` no log via loop de grep, como nos boot-tests anteriores; depois enviar comandos pela fifo.)

- [ ] **Step 3: Comandos de smoke no live**

Enviar pela fifo (uma linha por `printf '...' > /tmp/sm-in`):
```text
ls /etc/tuios-installer/
tuios-instalar --version
nixos-install --help | grep -E "no-root-passwd|no-channel-copy"
nmcli -t -f STATE general || true
```
Expected:
- `/etc/tuios-installer/` contém `configuration.nix tuios-session.nix tuios-env.nix nixpkgs-path pkg-path? tuios-installer-pkg.nix`
- `tuios-instalar 1.0.0`
- flags do nixos-install confirmadas (se ausentes → ajustar `instalar_sistema` na Task 6 e rebuild)
- estado NM não-fatal (`unknown`/`connected` aceitável)

- [ ] **Step 4: Commit (se ajustes de flags foram necessários)**

```bash
git add -A
git commit -m "[TEST] — Smoke test da ISO com tuios-instalar embarcado (flags nixos-install validadas)"
```

---

### Task 10: `scripts-assert/install.assert.sh` — teste ponta a ponta (T1+T2) + Justfile

**Files:**
- Create: `scripts-assert/install.assert.sh`
- Modify: `Justfile` (target `test-install`)

**Interfaces:**
- Consumes: ISO (Task 9), OVMF (`/usr/share/OVMF/OVMF_{CODE,VARS}_4M.fd`), `qemu-img`, `qemu-system-x86_64`.
- Produz tokens: `INSTAL-OK` (via instalador), `INSTAL-TEST-OK` (fim do script, exit 0).

- [ ] **Step 1: Escrever o script**

```bash
#!/usr/bin/env bash
# install.assert.sh — T1: instalação automatizada em QEMU; T2: boot do disco instalado.
set -euo pipefail
cd "$(dirname "$0")/.."

ISO="${ISO:-}"
if [ -z "$ISO" ]; then
  for c in result/iso/*.iso /tmp/tuios-prime-iso-oficial/*.iso; do
    [ -f "$c" ] && ISO="$c" && break
  done
fi
[ -f "${ISO:-}" ] || { echo "ISO não encontrada — rode: just iso"; exit 1; }

OVMF_CODE="${OVMF_CODE:-/usr/share/OVMF/OVMF_CODE_4M.fd}"
OVMF_VARS="${OVMF_VARS:-/usr/share/OVMF/OVMF_VARS_4M.fd}"
[ -f "$OVMF_CODE" ] && [ -f "$OVMF_VARS" ] || { echo "OVMF ausente (pacote ovmf)"; exit 1; }

WORK=$(mktemp -d /tmp/tuios-install-test.XXXXXX)
DISK="$WORK/disk.qcow2"
qemu-img create -qf qcow2 "$DISK" 8G >/dev/null
mkfifo "$WORK/in"

QPID=""
cleanup() { [ -n "$QPID" ] && kill "$QPID" 2>/dev/null || true; }
trap cleanup EXIT

boot() { # $1 = iso ou "null"
  cp "$OVMF_VARS" "$WORK/vars.fd"
  local cdrom=()
  [ "$1" != "null" ] && cdrom=(-cdrom "$1")
  qemu-system-x86_64 -M q35 -m 2048M -display none -serial stdio \
    -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
    -drive if=pflash,format=raw,file="$WORK/vars.fd" \
    -drive file="$DISK",format=qcow2,if=virtio \
    "${cdrom[@]}" \
    -netdev user,id=n0 -device virtio-net-pci,netdev=n0 \
    < "$WORK/in" > "$WORK/log" 2>&1 &
  QPID=$!
}

esperar() { # $1=padrão $2=timeout_s
  local i
  for i in $(seq 1 "$2"); do
    grep -a -q "$1" "$WORK/log" 2>/dev/null && return 0
    kill -0 "$QPID" 2>/dev/null || { echo "QEMU morreu — veja $WORK/log"; return 1; }
    sleep 1
  done
  echo "TIMEOUT esperando: $1 — últimas linhas:"
  tail -20 "$WORK/log" | tr -d '\000-\010\013-\037'
  return 1
}

enviar() { printf '%s\n' "$1" > "$WORK/in"; }

echo "[1/2] Boot da ISO + instalação automatizada..."
boot "$ISO"
esperar "root@tuios-prime:~#" 600
sleep 3
enviar "tuios-instalar --auto --disco /dev/vda --sem-rede --aceitar-tudo"
esperar "INSTAL-OK" 1800
kill "$QPID" 2>/dev/null; wait "$QPID" 2>/dev/null || true; QPID=""

echo "[2/2] Boot do disco instalado..."
boot null
esperar "tuios-prime login:" 900
sleep 5
enviar "systemctl is-active tuios-session"
esperar "active" 60
sleep 1
enviar "hostname; advplc --version 2>&1 | head -1"
esperar "advplc" 30

kill "$QPID" 2>/dev/null || true
grep -a -q "INSTAL-OK" "$WORK/log" || { echo "FALHA: INSTAL-OK ausente"; exit 1; }
echo ""
echo "============================================"
echo "   INSTAL-TEST-OK (T1 instalação + T2 boot)"
echo "============================================"
rm -rf "$WORK"
```

- [ ] **Step 2: Tornar executável + sintaxe**

```bash
chmod +x scripts-assert/install.assert.sh
bash -n scripts-assert/install.assert.sh && echo SINTAXE-OK
```

- [ ] **Step 3: Adicionar target no Justfile (após `test-all`)**

```just
# Teste ponta a ponta do instalador (QEMU: ISO → disco → boot)
test-install:
    bash scripts-assert/install.assert.sh
```

- [ ] **Step 4: Rodar o teste completo**

Run: `just test-install`
Expected: `[1/2]...` → `INSTAL-OK` (≈3–15 min) → `[2/2]...` → banner `INSTAL-TEST-OK`, exit 0.
Diagnóstico: falhou → inspecionar o diretório `/tmp/tuios-install-test.*/log`.

- [ ] **Step 5: Regressão dos 8 asserts**

Run: `just test-all`
Expected: `TODOS OS TESTES PASSARAM! ✅`

- [ ] **Step 6: Commit**

```bash
git add scripts-assert/install.assert.sh Justfile
git commit -m "[TEST] — install.assert.sh: T1 instalação + T2 boot do disco em QEMU"
```

---

### Task 11: CI — job `iso-install` (condicional ao binário vendored)

**Files:**
- Modify: `nixos/advpp/default.nix` (fallback de `advppBin`)
- Modify: `.github/workflows/build.yml` (job condicional)

**Interfaces:**
- Produz: build puro (`--impure` só se `nixos/advpp/advplc` ausente) e CI que roda `test-install` **apenas quando** o binário existir no repo.

**Decisão de escopo (sinalizar ao operador):** o `advplc` tem **71MB**; vendoriá-lo em `nixos/advpp/advplc` torna o build 100% puro e habilita o CI completo (limite do GitHub por arquivo: 100MB — cabe). Sem ele, o job CI é pulado com aviso.

- [ ] **Step 1: `nixos/advpp/default.nix` — fallback**

```nix
  # Binário do compilador AdvPL: prioriza o binário versionado no repo
  # (build puro); fallback para o checkout local (exige --impure).
  advppBin ? (if builtins.pathExists ./advplc then ./advplc
              else /home/peder/Projetos/AdvPP/advplc),
```
(ajustar a assinatura existente — hoje é `advppBin ? /home/peder/Projetos/AdvPP/advplc`)

- [ ] **Step 2: Job condicional no workflow (adicionar após o job `build`)**

```yaml
  iso-install:
    name: ISO + Instalador (QEMU)
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4

    - name: Detectar binário AdvPP vendored
      id: advpp
      run: echo "ok=$([ -f nixos/advpp/advplc ] && echo true || echo false)" >> "$GITHUB_OUTPUT"

    - name: Instalar Nix
      if: steps.advpp.outputs.ok == 'true'
      uses: cachix/install-nix-action@v27
      with:
        extra_nix_config: |
          experimental-features = nix-command flakes

    - name: Dependências (QEMU + OVMF)
      if: steps.advpp.outputs.ok == 'true'
      run: |
        sudo apt update
        sudo apt install -y qemu-system-x86 qemu-utils ovmf

    - name: Build da ISO
      if: steps.advpp.outputs.ok == 'true'
      run: nix build .#nixosConfigurations.iso.config.system.build.isoImage --print-out-paths --out-link result

    - name: Teste do instalador (T1+T2)
      if: steps.advpp.outputs.ok == 'true'
      run: bash scripts-assert/install.assert.sh

    - name: Aviso (sem AdvPP vendored)
      if: steps.advpp.outputs.ok != 'true'
      run: echo "::notice::Job ISO pulado — binário nixos/advpp/advplc não versionado (71MB)"
```

- [ ] **Step 3: Validar YAML**

Run: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/build.yml')); print('YAML-OK')"`
Expected: `YAML-OK`.

- [ ] **Step 4: Push + acompanhar CI**

```bash
git add nixos/advpp/default.nix .github/workflows/build.yml
git commit -m "[CI] — Job iso-install condicional + fallback pura do advppBin"
git push origin main
gh run list --branch main --limit 2
```
Expected: job `Build e Testes` verde; `ISO + Instalador` verde (com vendored) ou notice (sem).

---

### Task 12: Docs do usuário final (PT-BR)

**Files:**
- Create: `docs/instalacao/instalar.md` — guia completo: queimar pendrive → boot → wizard passo a passo (5 etapas, com o que aparece em cada tela) → pós-boot (WiFi automático, `nixos-rebuild switch`)
- Create: `docs/instalacao/checklist-wifi.md` — checklist manual de validação do WiFi no Chromebook (ver SSID, conectar, reboot, reconexão automática)
- Modify: `README.md` — seção "Instalar" com 3 comandos
- Modify: `docs/instalacao/build-iso.md` — adicionar `just iso` e `just test-install`

**Interfaces:**
- Consome: comportamento real validado nas Tasks 9–10 (usar os textos exatos do wizard).

- [ ] **Step 1: Escrever os 2 docs novos** (conteúdo: fluxo do wizard com as caixas reais de `ui_msg`/`ui_menu`/`ui_confirma_texto` do Task 7; comando de queimar `dd` já existente em build-iso.md; checklist WiFi com comandos `nmcli device wifi list`, `systemctl is-active NetworkManager`, `nmcli connection show`)

- [ ] **Step 2: Atualizar README + build-iso.md**

- [ ] **Step 3: Commit**

```bash
git add docs/instalacao/ README.md
git commit -m "[DOC] — Guia do instalador + checklist WiFi (usuário final, PT-BR)"
```

---

### Task 13: Entrega final — ISO + evidências + mem0

**Files:**
- Modify: nenhum (execução/evidência)

- [ ] **Step 1: Build final da ISO**

Run: `nix build --impure .#nixosConfigurations.iso.config.system.build.isoImage --print-out-paths --out-link result`
Expected: caminho da ISO; registrar o SHA-256 (`sha256sum`).

- [ ] **Step 2: Bateria completa de validação**

Run: `just test-all && just test-install`
Expected: `TODOS OS TESTES PASSARAM! ✅` + `INSTAL-TEST-OK`.

- [ ] **Step 3: Gravação no pendrive do operador**

Passar o comando pronto (o operador digita a senha):
```bash
sudo dd if=result/iso/nixos-*.iso of=/dev/sdX bs=4M status=progress conv=fsync && sync
```
(Confirmar o dispositivo com `lsblk` antes — Lei 1: parada + descrição + confirmação.)

- [ ] **Step 4: Persistir na mem0 cross-agent**

`mem0_add` (fact): instalador tuios-instalar embarcado na ISO — arquitetura (installer/lib/*, etc tuios-installer, storePath template), flags validadas do nixos-install, resultado dos testes, caminho da ISO final + sha256.

- [ ] **Step 5: Push final + CI verde**

```bash
git add -A && git commit -m "[ISO] — Entrega: ISO com instalador tuios-instalar validado ponta a ponta" && git push origin main
gh run watch --exit-status
```

---

## Self-Review (writing-plans)

1. **Spec coverage:** D1–D6 ✓ (Tasks 7/5/3–7/2/10/1); WiFi ✓ (T5); particionamento ✓ (T4); offline-first ✓ (T2+T6); persistência NM ✓ (T5+T6); testes T1–T5 ✓ (T10/T11/T12); docs ✓ (T12); YAGNI ✓ (nada fora do escopo listado).
2. **Placeholder scan:** Task 8 Step 3 contém raciocínio de design embutido ("NÃO — o eval é...") — resolvido no próprio passo com a solução final (`tuios-installer-pkg.nix` via storePath). Sem TBD/TODO remanescentes; Task 9 repete comandos concretos (não "semelhante a").
3. **Type consistency:** `instalar_executar $disco`, `P1/P2/P3`, `discos_candidatos` (tag|desc), `rede_fluxo` (rc 0/1), `UI` (1/0), tokens `INSTAL-OK`/`INSTAL-TEST-OK` — consistentes entre Tasks 3→10. Entry usa exatamente as assinaturas produzidas.
4. **Correção aplicada:** o template não pode referenciar `../installer` no eval do destino — resolvido com embed `tuios-installer-pkg.nix` (storePath, contexto preservado).
