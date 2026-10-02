# Spec: Instalador tuiOS-Prime (tuios-instalar)

**Data:** 2026-10-02 · **Status:** Aprovado pelo operador (seção a seção)
**Escopo:** Assistente de instalação PT-BR embarcado na ISO NixOS do tuiOS-Prime,
com configuração de WiFi e persistência do sistema em disco.

## 1. Contexto

A ISO tuiOS-Prime boota em hardware real (validada em Chromebook, 4 CPUs,
3,3GB RAM) e inicia a sessão tuiOS automaticamente. Duas lacunas impedem o
uso por usuário final:

1. **Rede:** o adaptador WiFi aparece no sistema, mas não há fluxo guiado de
   conexão (a ISO minimal só expõe `wpa_supplicant` cru).
2. **Persistência:** não há instalador — o sistema vive só no pendrive.

## 2. Decisões aprovadas

| # | Decisão | Escolha |
|---|---------|---------|
| D1 | Destino do disco | Genérico — instalador pergunta qual disco (C) |
| D2 | WiFi | Interface detectada; falta fluxo/UI (A) |
| D3 | UX | Comando `tuios-instalar` com assistente visual `dialog` (C) |
| D4 | Escopo de config | WiFi salvo + locale + autologin root + particionamento + boot (a+b+c+d+e) |
| D5 | Teste | QEMU primeiro com disco virtual; WiFi manual no hardware (A) |
| D6 | Abordagem | Script `dialog` + ferramentas NixOS nativas (1) |

## 3. Arquitetura

```
installer/
├── tuios-instalar      # entry point (bash), parseia flags, abre wizard
├── lib/
│   ├── ui.sh           # caixas dialog: menu, senha, progresso, erro, log
│   ├── rede.sh         # nmcli: varrer SSID, conectar, verificar, fallback
│   ├── disco.sh        # listar discos seguros, confirmar APAGAR, particionar
│   ├── locale.sh       # abnt2 + pt_BR + America/Sao_Paulo
│   └── instalar.sh     # mount → generate-config → nixos-install → bootloader
└── README.md
```

**Integração ISO (`nixos/iso.nix`):**
- `networking.networkmanager.enable = true` (live e instalado)
- pacote `tuios-instalar` (com `dialog`, `nmcli`, `parted`, `dosfstools`,
  `e2fsprogs`, `gptfdisk` no closure) em `environment.systemPackages`
- **Flake embutida** em `/etc/tuios-flake/` com input `nixpkgs` reescrita
  para o `path:` do source já presente na store da ISO → instalação offline
  total (closure completo já no pendrive)

**Dois modos (lógica separada da UI):**
- Interativo: `tuios-instalar` (wizard de 5 etapas)
- Automático (teste/CI): `tuios-instalar --auto --disco /dev/vXX
  [--sem-rede] [--aceitar-tudo]`

## 4. Wizard — 5 etapas

1. **Pré-requisitos** — root, UEFI detectado, disco alvo existe, aborta cedo
2. **Rede** — `nmcli radio wifi on`; `nmcli device wifi list --rescan yes`;
   menu de SSIDs; senha via caixa (SSID oculto opcional); verificação de
   conexão; opção "Continuar sem rede" (instalação segue offline)
3. **Disco** — lista apenas discos não-usados pelo live (esconde o disco de
   `/` e `/boot`); escolha + confirmação digitando `APAGAR`
4. **Resumo** — plano completo (partições, locale, boot, rede) → confirma
5. **Instalação** — barra de progresso; log em `/var/log/tuios-install.log`;
   caixa final: Reiniciar / Voltar ao shell

## 5. Particionamento (automático, GPT+UEFI)

```
p1  EFI System  512 MiB  FAT32   /boot
p2  swap       2048 MiB  linux-swap
p3  root       resto     ext4    /
```

- `parted` + `mkfs.fat`/`mkfs.ext4`/`mkswap`
- Duas camadas de confirmação (seleção + digitar `APAGAR`)
- Sem `rm` selvagem: `wipefs`/`parted` somente no disco confirmado

## 6. Instalação

```bash
mount /dev/Xp3 /mnt && mount /dev/Xp1 /mnt/boot && swapon /dev/Xp2
nixos-generate-config --root /mnt --no-hardware   # + merge do template
# template do destino (= iso.nix com locale):
#   autologin root, tuios-session, NM, pt_BR/abnt2/SP
nixos-install --root /mnt --flake /etc/tuios-flake#instalado --no-root-passwd
bootctl install --esp-path /mnt/boot   # UEFI; GRUB fallback se BIOS
# rede: copiar /etc/NetworkManager/system-connections/* → /mnt/etc/...
```

**Erros:** `trap` por etapa → caixa com comando falho + caminho do log;
opções Tentar de novo / Voltar ao shell; cleanup de montagens no `trap EXIT`.

## 7. Persistência

- Perfil WiFi (perm 600) copiado para o destino → reconecta no boot
- Sistema instalado nasce com NetworkManager habilitado (declarativo)
- Autologin root + sessão tuiOS idênticos ao live

## 8. Testes

| # | Teste | Comando | Sucesso |
|---|-------|---------|---------|
| T1 | Instalação automatizada | `just test-install` | `INSTAL-OK` + 3 partições |
| T2 | Boot do disco instalado | (parte do T1) | login root + tuios-session `active` |
| T3 | Regressão kernel | `just test-all` | 8/8 |
| T4 | WiFi real | checklist manual (docs) | reconecta no boot |
| T5 | CI | job `test-install` | verde |

## 9. Fora de escopo (YAGNI)

- Atualizador/upgrade pós-instalação (uso `nixos-rebuild` manual)
- Criptografia LUKS (futuro)
- Dual-boot/GRUB menu com outros SOs
- Wizard gráfico X11/Wayland
- Suporte a MBR/BIOS legado além do fallback GRUB básico
- Download de firmware extra (o driver WiFi já funciona no live)

## 10. Definition of Done

1. `just iso` gera imagem com instalador embutido
2. Pendrive boota → `tuios-instalar` completo em QEMU (T1+T2) e no Chromebook
3. Sistema instalado reconecta no WiFi sozinho e inicia tuiOS
4. `just test-all` 8/8 + CI verde
5. Docs: `docs/instalacao/instalar.md` (guia do usuário final) +
   `checklist-wifi.md`
