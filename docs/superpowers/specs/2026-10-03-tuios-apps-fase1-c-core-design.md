# Spec — tuios-apps (Nível C, Fase 1: c-core)

> **Data:** 2026-10-03 · **Status:** design aprovado pelo operador · **Escopo:** sub-projeto `c-core`
> **Relacionado:** Nível C = "apps AdvPL de primeira classe no tuiOS"

## 1. Contexto e visão

O tuiOS-Prime hoje tem `advplc` (AdvPP v4.4.0) no PATH e `ADVPP_DB` configurado,
mas **nenhum conceito de "app"**: não há como empacotar, descobrir, listar ou
disparar programas AdvPL como unidades nomeadas. O Nível C introduz esse
conceito. A Fase 1 (`c-core`) entrega o **núcleo CLI testável**; a Fase 2
(`c-tui`) entrega menu TUI, exemplos na ISO e documentação.

**Decisões do brainstorm (registradas):**
1. App = diretório + manifesto `tuios-app.toml` (independente do AdvPP)
2. Interface primária = **CLI**; menu TUI é camada fina da Fase 2
3. Store mínimo = **local**: exemplos na ISO + dir do usuário + tarball
4. Stack = **Python 3 + tomllib (stdlib) + dialog** (opção B)

## 2. Escopo

**Dentro (Fase 1):**
- F1 Manifesto `tuios-app.toml` com validação estrita
- F2 Discovery em roots de sistema e usuário
- F3 CLI: `listar/info/rodar/validar`
- F4 Ciclo de pacote: `empacotar/adicionar/remover`
- F7 Testes: pytest + `apps.assert.sh` + smoke no QEMU
- Empacotamento Nix (`nixos/apps/tuios-apps.nix`) na ISO e no destino

**Fora (Fase 2 `c-tui`):** menu dialog (F5), exemplos na ISO (F6), guia
`docs/apps/` (F8), cópia de apps pelo instalador ao destino.
**Futuro (Fase 3):** origem git, índice remoto (store HTTP).

## 3. Requisitos funcionais

| ID | Requisito | Critério de aceite |
|----|-----------|--------------------|
| R1 | Manifesto TOML válido é reconhecido | campos obrigatórios `nome, versao, descricao, entry`; slug `^[a-z0-9][a-z0-9-]{0,40}$`; versão `N.N.N`; entry relativo existente com ext `.prw`/`.tlpp` |
| R2 | Chaves desconhecidas no manifesto → erro | exit 2, mensagem PT-BR apontando a chave |
| R3 | Discovery varre sistema e usuário | roots padrão `/opt/tuios/apps` e `${XDG_DATA_HOME:-~/.local/share}/tuios/apps`; env `TUIOS_APPS_SYSTEM`/`TUIOS_APPS_USER` sobrepõem (testes) |
| R4 | Subdir sem manifesto → aviso, não erro | app pulado, stderr com aviso, exit 0 |
| R5 | Colisão de nome: usuário sombra sistema | listagem mostra o do usuário com origem `usuario` |
| R6 | `listar` texto alinhado e `--json` | JSON estável: `nome, versao, descricao, categoria, origem, path` (`categoria` ausente → `null`); ordenado por nome; sem apps → lista `[]` (JSON) ou stdout vazio (texto), exit 0 |
| R7 | `rodar <nome>` executa via advplc | cwd = dir do app; `advplc run <entry>` + args após `--`; exit code propagado; **sem `shell=True`** |
| R8 | `advplc` ausente → exit 4 | mensagem clara PT-BR (dependência) |
| R9 | `empacotar <dir>` gera tar.gz íntegro | prefixo `nome/`, inclui `tuios-app.toml` + `SHA256SUMS`; exclui `.git`, `__pycache__`, `*.pyc` |
| R10 | `adicionar` valida antes de instalar | ordem: magic gzip → prefixo único → rejeita `..`/absoluto → checksum → manifesto; destino padrão = dir do usuário (`--sistema` → `/opt/tuios/apps`); falha = nada instalado |
| R11 | `remover` | remove do dir do usuário por padrão; `--sistema` remove de `/opt/tuios/apps` e exige root (senão exit 1 com mensagem) |
| R12 | `info <nome>` | mostra todos os campos + origem + path |
| R13 | `validar [dir]` | valida manifesto do dir informado (ou cwd); exit 0/2 |

## 4. Interface CLI

```
tuios-apps listar [--json]
tuios-apps info <nome>
tuios-apps rodar <nome> [-- args...]
tuios-apps validar [dir]
tuios-apps empacotar <dir> [-o saida.tar.gz]
tuios-apps adicionar <tarball> [--sistema]
tuios-apps remover <nome> [--sistema]
tuios-apps --version
```

**Exit codes:** `0` sucesso · `1` erro geral · `2` manifesto/pacote inválido ·
`3` app não encontrado · `4` dependência ausente.

**Contrato de saída:** sucesso → stdout (dados); erro → `stderr` no formato
`tuios-apps: erro: <msg>`; nenhum stacktrace para erro de uso
(`TUIOS_APPS_DEBUG=1` habilita traceback). Sem ANSI quando
`NO_COLOR`/não-TTY.

## 5. Modelo de dados

**Manifesto `tuios-app.toml`** (chaves de nível superior, tabela única):

```toml
nome = "ola-tuios"          # obrigatório — slug (R1)
versao = "1.0.0"            # obrigatório — N.N.N
descricao = "Primeiro app"  # obrigatório — 1..120 chars
entry = "ola.prw"           # obrigatório — relativo, existe, .prw/.tlpp
categoria = "exemplo"       # opcional
autor = "Peder"             # opcional
```

**App (modelo interno):** `nome, versao, descricao, entry, categoria, autor,
origem ("sistema"|"usuario"), path`.

**Pacote:** tar.gz prefixado com `nome/` + `SHA256SUMS` (sha256 `<hex>  <rel>`).

## 6. Arquitetura

```
apps/tuios-apps/
├── tuios-apps              # entrypoint executável
└── tuiosapps/
    ├── manifest.py    → tomllib + validação → Manifest | ErroManifesto
    ├── discovery.py   → roots → [App] (somente leitura)
    ├── package.py     → empacotar / adicionar / remover
    ├── runner.py      → subprocess advplc (shell=False)
    └── cli.py         → argparse + exit codes + --json
nixos/apps/tuios-apps.nix  # buildPythonApplication (stdlib only)
```

- Módulos sem dependência de CLI; `cli.py` orquestra; `menu` (Fase 2) consumirá
  os mesmos módulos.
- `buildPythonApplication` com `python3` ≥ 3.11 (tomllib stdlib) — **zero
  dependências PyPI**; Nix faz substituição do shebang.

## 7. Segurança

- Extração de tarball: rejeitar entradas com `..`, caminhos absolutos,
  symlinks apontando fora do destino; staging antes do move final.
- Execução sem `shell=True`; args passados como lista.
- `--sistema` requer root (verificação por `os.geteuid()`).

## 8. Testes e critérios de aceite (F7)

| Camada | Conteúdo |
|--------|----------|
| pytest | por módulo: manifestos válidos/inválidos (R1,R2), discovery (R3-R5), JSON (R6), pacote seguro (R9,R10 — incl. tarballs maliciosos), runner (R7,R8), exit codes |
| `scripts-assert/apps.assert.sh` | ciclo real em tmpdir: empacotar → adicionar → listar → rodar → remover; padrão de saída `APPS-ASSERT-OK`; entra no `just test-all` |
| QEMU | smoke no `install.assert.sh` T2: `tuios-apps listar --json` parseável no sistema instalado |
| CI | workflow existente passa a rodar pytest + asserts |

**Aceite da Fase 1:** todos os R verificados por teste automatizado; `just
test-all` e `just test-install` verdes; `tuios-apps` disponível na ISO.

## 9. Fronteiras com outras fases

- **c-tui (Fase 2):** menu dialog reusa `discovery/manifest/runner`; exemplos
  em `apps/exemplos/` copiados para `/opt/tuios/apps` via Nix; guia em `docs/`.
- **Fase 3:** `adicionar` ganhará origem git/remota sem mudar manifesto
  (manifesto é a interface comum).
