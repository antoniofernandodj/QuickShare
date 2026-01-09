run:
	docker compose down
	docker compose up --build -d
	docker compose logs -f api

stop:
	docker compose down


build:
	docker builder build --file Dockerfile --tag antoniofernandodj/quickshare:latest .

push:
	docker push antoniofernandodj/quickshare:latest