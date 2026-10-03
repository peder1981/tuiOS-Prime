# Instalar o tuiOS-Prime em disco

> Guia completo para o usuário final. O instalador `tuios-instalar` vem
> embutido na ISO e faz tudo: WiFi, particionamento, sistema e boot.

## 📋 Pré-requisitos

- Pendrive USB com **8GB+** (a ISO tem ~1,2GB)
- Máquina com **UEFI** (Chromebook em modo desenvolvedor, PC moderno)
- Disco de destino com **~6GB livres** (o sistema ocupa ~3,5GB)

## 💾 1. Queimar a ISO no pendrive

```bash
# Descobrir o dispositivo do pendrive (confirmar com lsblk!)
lsblk

# Gravar (SUBSTITUA /dev/sdX pelo dispositivo certo — APAGA tudo nele)
sudo dd if=result/iso/nixos-*.iso of=/dev/sdX bs=4M status=progress conv=fsync
sync
```

> ⚠️ **Lei de ouro:** confirme o dispositivo com `lsblk` antes do `dd`.
> Gravar no disco errado destrói seus dados.

## 🖥️ 2. Boot pelo pendrive

1. Ligue o Chromebook segurando `Ctrl+U` (ou selecione o pendrive no boot do PC)
2. O tuiOS-Prime inicia **automaticamente** como root (autologin)
3. O **assistente de instalação abre sozinho** no boot — siga as 5 etapas
   na tela (veja a próxima seção)

> Se você **cancelar** o assistente (ESC/Cancelar/NAO), a sessão tuiOS abre
> normalmente e você pode reexecutá-lo a qualquer momento:

```bash
tuios-instalar
```

## ✨ 3. O assistente — passo a passo

O assistente tem **5 etapas** e guia tudo em diálogos:

### Etapa 1 — Boas-vindas e escolha do disco

1. **Tela inicial** — aviso de que o disco será apagado e resumo dos padrões:
   ```
   locale pt_BR.UTF-8 · teclado br-abnt2 · fuso America/Sao_Paulo
   ```
2. **Menu de discos** — lista todos os discos utilizáveis (o pendrive e o
   disco de instalação do sistema ao vivo são excluídos automaticamente).
   Escolha o disco de destino com as setas + `Enter`.
3. **Confirmação** — digite exatamente **`APAGAR`** para prosseguir.
   Qualquer outra coisa cancela com segurança.

> Se a máquina não estiver em UEFI, o instalador aborta com aviso.

### Etapa 2 — Rede (WiFi)

1. O instalador varre as redes e mostra o **menu de WiFi** com sinal e segurança.
2. Escolha a rede, digite a senha (até **3 tentativas** por rede).
3. Sucesso: `Conectado a <SSID>!`
4. **Cancelar o menu** ou falhar as tentativas = instalação **offline**
   (funciona normalmente — tudo já está na ISO). O WiFi pode ser
   configurado depois, no sistema instalado.

> Sem WiFi? O passo seguinte informa que a instalação segue offline.

### Etapa 3 — Resumo e confirmação

Caixa com tudo decidido — confira e responda `Sim`:

```
Disco: /dev/sda
Partições: EFI 512M · swap 2G · root ext4 (resto)
Rede: conectada | não conectada
Boot: systemd-boot (UEFI)
locale pt_BR.UTF-8 · teclado br-abnt2 · fuso America/Sao_Paulo
```

### Etapa 4 — Instalação

Barra de progresso mostra as 5 sub-etapas:

| # | Etapa | O que acontece |
|---|-------|----------------|
| 1 | Particionando | GPT + EFI 512MiB (FAT32) + swap 2048MiB + root ext4 |
| 2 | Montando discos | Partições montadas em `/mnt` + swap ativado |
| 3 | Configurando | `nixos-generate-config` + locale BR + sessão tuiOS + perfis de rede |
| 4 | Instalando | `nixos-install` copia o sistema (**offline**, direto da ISO) |
| 5 | Bootloader | systemd-boot na EFI (verificado/instalado) |

### Etapa 5 — Conclusão

```
Instalação concluída com sucesso!
Remova o pendrive e reinicie.
No próximo boot o tuiOS inicia automaticamente.
```

**Reinicie:** `reboot` (remova o pendrive antes).

## 🎉 4. Pós-instalação

No primeiro boot do disco instalado:

- **Autologin root** → sessão `tuios-session` inicia sozinha
- **WiFi**: reconecta automaticamente (perfil copiado da instalação)
- **Rede gerenciada** pelo NetworkManager

Após alterar qualquer coisa em `/etc/nixos/`:

```bash
nixos-rebuild switch
```

### Comandos úteis

```bash
tuios-instalar --help          # ajuda e todas as opções
nmcli device wifi list         # redes visíveis
nmcli connection show          # perfis salvos
systemctl is-active tuios-session   # deve responder: active
advplc --version               # compilador AdvPL (deve responder: advplc dev)
```

## 🤖 Modo automático (testes/CI)

```bash
tuios-instalar --auto --disco /dev/vda --sem-rede --aceitar-tudo
```

- `--auto` — interface em texto puro (sem dialog)
- `--disco X` — disco alvo sem menu
- `--sem-rede` — pula o WiFi
- `--aceitar-tudo` — não pede confirmações (⚠️ apaga o disco direto)

Saída esperada ao final: **`INSTAL-OK`**.

## 🧪 Validação automatizada (desenvolvedores)

```bash
just iso            # build da ISO
just test-install   # T1: instala em QEMU · T2: boota o disco instalado
```

Sucesso = banner `INSTAL-TEST-OK`. Logs em `/tmp/tuios-install-test.*/log`.

## 🔧 Solução de problemas

| Sintoma | Causa provável | Solução |
|---------|----------------|---------|
| "Nenhum disco utilizável" | disco com partições montadas | desmonte ou use outro disco |
| "Disco hospeda o sistema ao vivo" | escolheu o pendrive | escolha o disco interno |
| "Modo UEFI não detectado" | boot legado | ative UEFI no firmware |
| Falha na Etapa 4 | disco cheio/memória | veja `/var/log/tuios-install.log` |
| WiFi não conecta | driver/senha | instale offline e configure depois ([checklist WiFi](checklist-wifi.md)) |

---

**Última atualização:** Outubro 2026
