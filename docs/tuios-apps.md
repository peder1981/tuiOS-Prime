# tuios-apps — apps AdvPL de primeira classe (Nível C)

> Guia completo do gerenciador de apps do tuiOS-Prime.
> Spec: [Fase 1 (c-core)](superpowers/specs/2026-10-03-tuios-apps-fase1-c-core-design.md) ·
> [Fase 2 (c-tui)](superpowers/specs/2026-10-03-tuios-apps-fase2-c-tui-design.md)

## O que é

O **tuios-apps** trata programas AdvPL como **apps nomeados**: um diretório
com um manifesto `tuios-app.toml`. Dá para listar, inspecionar, executar,
empacotar e instalar — pela linha de comando ou por um menu interativo.

## O manifesto `tuios-app.toml`

```toml
nome = "ola-tuios"          # obrigatório — slug: ^[a-z0-9][a-z0-9-]{0,40}$
versao = "1.0.0"            # obrigatório — N.N.N
descricao = "Primeiro app"  # obrigatório — 1..120 caracteres
entry = "ola.prw"           # obrigatório — relativo, existe, .prw/.tlpp
categoria = "exemplo"       # opcional
autor = "Peder Munksgaard"  # opcional
```

Chave desconhecida, entry fora do diretório ou versão fora do padrão →
**exit 2** com a chave apontada.

## CLI

```
tuios-apps listar [--json]              # apps instalados (ordenados)
tuios-apps info <nome>                  # detalhes de um app
tuios-apps rodar <nome> [-- args...]    # executa via advplc run (cwd = app)
tuios-apps validar [dir]                # valida manifesto do diretório
tuios-apps empacotar <dir> [-o saida]   # gera <app>.tar.gz com SHA256SUMS
tuios-apps adicionar <tarball> [--sistema]   # instala (valida tudo antes)
tuios-apps remover <nome> [--sistema]   # remove (sistema exige root)
tuios-apps menu                         # menu interativo (dialog)
tuios-apps --version
```

**Exit codes:** `0` sucesso · `1` erro geral · `2` manifesto/pacote inválido ·
`3` app não encontrado · `4` dependência ausente (`advplc`/`dialog`).

## Menu interativo

`tuios-apps menu` abre um menu com `dialog`:

- **Listar apps** → escolher um app → ver detalhes, rodar ou remover
- **Instalar pacote .tar.gz** → escolhe o arquivo → validação completa
- **Sair**

O menu opera o diretório do **usuário**; operações no root de sistema
(`/opt/tuios/apps`) ficam na CLI com `--sistema` (root). Sem `dialog`
instalado: exit 4. `TUIOS_DIALOG` sobrepõe o binário (testes).

## Roots e sombra

| Root | Caminho |
|------|---------|
| sistema | `/opt/tuios/apps` (exemplos de fábrica; `--sistema` grava aqui) |
| usuário | `${XDG_DATA_HOME:-~/.local/share}/tuios/apps` (padrão) |

Mesmo nome nos dois roots → **o do usuário sombreia o do sistema**.
As envs `TUIOS_APPS_SYSTEM`/`TUIOS_APPS_USER` sobrepõem os caminhos (testes).

## Ciclo de pacote

1. `tuios-apps empacotar minha-pasta` → `nome-1.0.0.tar.gz`
   (prefixo `nome/`, inclui manifesto + `SHA256SUMS`; exclui `.git`,
   `__pycache__`, `*.pyc`)
2. `tuios-apps adicionar nome-1.0.0.tar.gz` → valida em staging:
   gzip → prefixo único → sem `..`/absoluto/symlink externo → checksum →
   manifesto; **só então** instala (falha = nada instalado)
3. `tuios-apps remover nome`

## Origens remotas e índice

```bash
# Download direto (http/https/file://) — timeout de 30 s
tuios-apps adicionar https://exemplo/ola-tuios-1.0.0.tar.gz

# Repositório git — git clone --depth 1 (120 s, sem prompt de credencial)
tuios-apps adicionar https://exemplo/meu-app.git

# Índice remoto (indice.toml) — grava em ${XDG_DATA_HOME:-~/.local/share}/tuios/
tuios-apps atualizar-indice https://exemplo/indice.toml
tuios-apps buscar termo [--json]
tuios-apps instalar <nome> [--sistema]   # baixa e confere o sha256 do índice
```

O índice é um `indice.toml` com entradas `[[app]]` (`nome`, `versao`,
`descricao`, `url`, `sha256` de 64 hex); gravação atômica e validação
completa — índice inválido **não** é gravado (exit 2). `instalar` confere o
sha256 do arquivo baixado contra o do índice antes de instalar (divergência =
exit 2, nada instalado).

**Rede só sob comando explícito:** `listar`/`info`/`rodar`/`validar`/
`empacotar`/`remover`/`menu` nunca acessam a rede. `git ausente` = exit 4.

## Exemplo de fábrica

A ISO traz o app **`ola-tuios`** em `/opt/tuios/apps` (copiado no boot via
tmpfiles — some se apagado, conteúdo do usuário nunca é sobrescrito) e o
instalador o leva para o disco destino. Confira com:

```
tuios-apps listar --json
```

## Desenvolvimento

```bash
just test-python   # pytest (suíte unitária)
just test-apps     # assert do ciclo completo em tmpdir
just test-all      # tudo (inclui os asserts de QEMU)
```

Fonte: `apps/tuios-apps/` (Python stdlib puro, zero deps PyPI);
empacotamento Nix em `nixos/apps/`.
