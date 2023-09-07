all: contrib-res-fetch trunk-serve

contrib-res-fetch:
	cd contrib-res && make

trunk-serve:
	trunk serve