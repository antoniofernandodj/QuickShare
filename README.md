# 📁 Quickshare

**Quickshare** é uma aplicação web para compartilhamento temporário de arquivos (24h), construída com **Rust**, **Axum** e **Yew**.  
Arquivos enviados são armazenados no **MongoDB GridFS** e podem ser baixados pelo navegador ou copiando o link de download.  

---

## ⚡ Funcionalidades

- Upload de arquivos pelo navegador  
- Download com nome original preservado  
- Expiração automática (TTL: 24h)  
- Lista de arquivos enviados  
- Copiar link de download rapidamente  
- Indicador de carregamento (spinner) durante upload/download  

---

## 🛠 Tecnologias

- Backend: Rust + Axum  
- Frontend: Yew + WASM  
- Banco: MongoDB GridFS  
- CORS: Tower HTTP  
- Requests HTTP no frontend: gloo-net  

---

## 🚀 Como rodar

### Pré-requisitos

- Rust >= 1.70  
- MongoDB rodando localmente ou remoto  

### Backend

```bash
# Defina a URI do MongoDB
export MONGO_URI="mongodb://localhost:27017"

# Compile e rode
cargo run
