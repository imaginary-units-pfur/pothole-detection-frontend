all: install tilesets run
install:
	wget https://github.com/consbio/mbtileserver/releases/download/v0.9.0/mbtileserver_v0.9.0_linux_amd64.zip -O mbtileserver_v0.9.0_linux_amd64.zip
	unzip -u mbtileserver_v0.9.0_linux_amd64.zip

tilesets:
	mkdir tilesets
	# planet
	#cd tilesets && wget -O - https://hot-qa-tiles-us-east-1.s3.amazonaws.com/latest.planet.mbtiles.gz | gzip -d - > planet.mbtiles
	# Russia
	cd tilesets && wget -O - https://hot-qa-tiles-us-east-1.s3.amazonaws.com/latest.country/russia.mbtiles.gz | gzip -d - > russia.mbtiles

run:
	./mbtileserver_v0.9.0_linux_amd64