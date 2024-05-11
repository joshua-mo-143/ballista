# Ballista: Query Your Notes
## Introduction
Ever wanted to query your Obsidian notes using ChatGPT? Now you can! This Rust web service lets you do exactly that, by ingesting your Obsidian notes as embeddings into a Qdrant database and then using ChatGPT for prompting.

## Usage
Usage assumes that your obsidian-git repo is in a private repo you own.

This service is primarily (currently) deployed through Shuttle. To deploy it, do the following:
- Run `cargo shuttle init --from joshua-mo-143/ballista` and follow the instructions
- Copy `Secrets.toml.example` to `Secrets.toml` and fill out the secrets. See the following below for an explanation:
  - `OPENAI_KEY`: An OpenAI API key for which you have funds on. 
  - `GITHUB_PERSONAL_ACCESS_TOKEN` - A GitHub Personal Access Token. You need at the very least `repo:read` permissions, since this is required to be able to download your obsidian-git repo if it's private.
  - `GITHUB_USERNAME` - Your username (or alternatively - someone else's username, if you're using their obsidian-git repo).
  - `GITHUB_REPO` - The repo you want to ingest. Note that only Markdown files are supported for now - Ballista will ignore anything else.
  - `QDRANT_URL` - The URL of your Qdrant database. Note that the port needs to be 6334 as `qdrant_client` utilises the gRPC URL. If you're just trying this out locally, you can leave it blank.
  - `QDRANT_API_KEY` - Your Qdrant API key. You can leave this blank if you're just running this locally.
  
## Features
- Downloads your Markdown files from a GitHub repo and ingests it into ChatGPT embeddings, then stores it in Qdrant.
- Supports Github webhooks at `/webhooks/github` for self-updating with no input required on your side.
- Uses an internal queue for resilient updating. If your update fails, simply send the webhook again!

## Isn't this just a ChatGPT wrapper?
Yes, but I don't want to pay exorbitant amounts just to be able to query my extremely small knowledge base. If you're interested in using this, I don't think you do either.
