all: contrib-res-fetch art-run trunk-serve

contrib-res-fetch:
	cd contrib-res && make

art-run:
	cd art && make

trunk-serve: check-db-permissions
	trunk serve

check-db-permissions:
	ls ./backend/database/database-data/* 2>&1 > /dev/null || echo "The backend's database directory is inaccessible, meaning trunk serve will fail. Trying to chown it to your user..." && sudo chown -R $$(whoami) ./backend/database