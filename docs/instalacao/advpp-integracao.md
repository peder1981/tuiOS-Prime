# Integracao AdvPP no tuiOS-Prime

> Status da simbiose entre tuiOS e AdvPP

## 📊 Status Atual

| Componente | Status | Notas |
|------------|--------|-------|
| **advplc (CLI)** | ✅ Integrado | Comando `advplc` disponivel no shell |
| **Package Nix** | ✅ Criado | `nixos/advpp/default.nix` |
| **ISO NixOS** | 🔄 Configurado | advplc sera incluso na ISO |
| **Kernel stub** | ✅ Implementado | Comandos disponiveis mas funcionam so na ISO |

## 🔧 Como Funciona

### No Kernel (Fase atual)
```
> advplc
AdvPP - Compilador AdvPL/TLPP
Usage: advplc <command> <file> [options]

Commands:
  run <file>          Compile and run
  compile <file> [-o <out>]  Compile to bytecode
  exec <bytecode>     Execute bytecode
  check <file>        Validate syntax
  serve <file> [--port <n>]  Web mode
  build <file> [-o <out>] [--gui]  Standalone build
  debug <file>        DAP debug server
  ast <file>          Print AST
  bytecode <file>     Print bytecode

Note: Full advplc support requires NixOS ISO (Fase 5)
```

### Na ISO NixOS (Fase 5)
Quando a ISO NixOS for construida, o `advplc` estara disponivel:

```bash
# Compilar e executar
advplc run hello.prw

# Compilar para bytecode
advplc compile hello.prw -o hello.bytecode

# Executar bytecode
advplc exec hello.bytecode

# Validar sintaxe
advplc check program.prw

# Modo web
advplc serve app.prw --port 9000 --watch

# Build standalone
advplc build app.prw -o app --gui
```

## 📦 Pacote Nix

O package esta em `nixos/advpp/default.nix`:

```nix
{ pkgs, ... }:

pkgs.stdenv.mkDerivation {
  pname = "advplc";
  version = "1.0.0";
  
  src = /home/peder/Projetos/AdvPP;
  
  nativeBuildInputs = [ pkgs.go ];
  buildInputs = [ pkgs.go ];
  
  GO111MODULE = "on";
  CGO_ENABLED = "0";
  
  buildPhase = ''
    go build -ldflags="-X main.version=${version}" -o advplc ./cmd/advplc
  '';
  
  installPhase = ''
    mkdir -p $out/bin
    mv advplc $out/bin/
  '';
}
```

## 🔗 Source AdvPP

- **Repositorio:** `/home/peder/Projetos/AdvPP`
- **Binario:** `advplc` (71MB, estatico, CGO=0)
- **Comando:** `go build -o advplc ./cmd/advplc`

## 📝 Próximos Passos (Fase 5)

1. **Build da ISO NixOS** com advplc incluido
2. **Associacao de arquivos** `.prw`/`.tlpp` → `advplc`
3. **Shell integration** para executar comandos AdvPP diretamente
4. **Documentacao** de exemplos e tutoriais

## 🚀 Testar Localmente

```bash
# Usar advplc diretamente
/home/peder/Projetos/AdvPP/advplc run /caminho/para/arquivo.prw

# Ou via Nix (quando ISO estiver pronta)
nix run .#advplc -- run hello.prw
```

---

**Ultima atualizacao:** Outubro 2026
**Responsavel:** Peder Munksgaard
