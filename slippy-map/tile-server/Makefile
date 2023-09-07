current_dir := $(shell pwd)

all: pbf database run
pbf:
	mkdir pbf
	# Russia
	wget -O pbf/russia-latest.osm.pbf https://download.geofabrik.de/russia-latest.osm.pbf

database:
	mkdir database
	docker run  -v $(current_dir)/pbf/russia-latest.osm.pbf:/data/region.osm.pbf  -v $(current_dir)/database/:/data/database/  overv/openstreetmap-tile-server  import

run:
	docker run -p 8080:80 -v $(current_dir)/database:/data/database -d overv/openstreetmap-tile-server run