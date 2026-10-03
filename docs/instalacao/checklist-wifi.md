# Checklist WiFi (Chromebook + tuiOS-Prime)

> Validação manual do WiFi no hardware real. O QEMU não tem dispositivo
> WiFi, então este checklist é feito no Chromebook.

## Pré-condições

- [ ] Sistema instalado e booted (ou ISO no pendrive)
- [ ] Logado como root (autologin)
- [ ] `rfkill list` sem `Soft blocked: yes` (se bloqueado: `rfkill unblock all`)

## 1. NetworkManager ativo

```bash
systemctl is-active NetworkManager
```

- [ ] Resposta: **`active`**

## 2. Interface WiFi visível

```bash
nmcli device status
```

- [ ] Há uma linha tipo `wlp2s0  wifi  disconnected`

Se `wifi` não aparecer:

```bash
nmcli radio wifi on
lsmod | grep -i ath   # driver Atheros comum em Chromebooks
dmesg | grep -i firmware | tail -5   # pediu firmware?
```

- [ ] ( ) Interface apareceu após `nmcli radio wifi on`

## 3. Redes visíveis

```bash
nmcli device wifi list
```

- [ ] O SSID da sua rede aparece na lista com sinal

## 4. Conectar (no sistema instalado, 2 formas)

**Pela CLI:**

```bash
nmcli device wifi connect "MEU-SSID" password "MINHA-SENHA"
nmcli connection show    # deve listar MEU-SSID com STATE ativo
```

**Pelo instalador (apenas na ISO live):** rodar `tuios-instalar` e escolher
a rede na Etapa 2 — o perfil é copiado para o sistema instalado.

- [ ] `ping -c 3 8.8.8.8` responde
- [ ] `ping -c 3 archlinux.org` responde (DNS)

## 5. Persistência após reboot

```bash
reboot
# após subir (autologin root):
nmcli connection show --active    # deve listar MEU-SSID
ping -c 3 8.8.8.8
```

- [ ] Reconectou **sozinho** sem digitar a senha

## 6. Reconfigurar do zero (se necessário)

```bash
nmcli connection delete "MEU-SSID" 2>/dev/null
nmcli device wifi rescan
nmcli device wifi connect "MEU-SSID" password "SENHA"
```

## 7. Sessão tuiOS + rede

```bash
systemctl is-active tuios-session    # active
```

- [ ] A sessão tuiOS sobe com rede pronta (NetworkManager Wait Online OK)

## Problemas comuns

| Sintoma | Diagnóstico | Ação |
|---------|-------------|------|
| `wifi` ausente no `device status` | driver/firmware | `dmesg \| grep -i firmware`; ver [troubleshooting](troubleshooting.md) |
| `rfkill` bloqueado | rádio bloqueada (soft blocked) | `rfkill unblock all` |
| conecta mas sem internet | DNS | `ping 8.8.8.8` ok? → `echo "nameserver 8.8.8.8" > /etc/resolv.conf` (gerenciado pelo NM normalmente) |
| não reconecta no reboot | perfil ausente | refazer etapa 4 — perfil deve estar em `/etc/NetworkManager/system-connections/` |
| senha errada | Etapa 2 do instalador | máx. 3 tentativas; rodar `tuios-instalar` de novo ou conectar via CLI |

---

**Última atualização:** Outubro 2026
