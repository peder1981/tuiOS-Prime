# Guia de Contribuição — tuiOS-Prime

> Como contribuir com o desenvolvimento

## 🎯 Primeiros Passos

### 1. Configurar Ambiente

```bash
# Instalar Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly
rustup default nightly
```

### 2. Clonar Repositório

```bash
git clone https://github.com/peder1981/tuiOS-Prime.git
cd tuiOS-Prime
```

## 📝 Convenções

### Commits

```
[FEAT] — nova funcionalidade
[FIX] — correção de bug
[REF] — refatoração
[DOC] — documentação
[TEST] — testes
```

### Código

- Use notação húngara: `c` (char), `n` (num), `l` (bool), `a` (array)
- Declarar todas as variáveis como `Local` no topo da função
- Documentar funções com comentários `///`

## 🧪 Testes

```bash
# Todos os testes
just test-all

# Teste específico
just test-shell
```

## 📋 Checklist de PR

- [ ] Código segue convenções
- [ ] Testes passam
- [ ] Documentação atualizada
- [ ] Commits atômicos

---

Obrigado por contribuir! 🚀
