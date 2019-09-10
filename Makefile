run-autobahn:
	docker run -it --rm \
		-v "${PWD}/config:/config" \
		-v "${PWD}/reports:/reports" \
		--network host \
		crossbario/autobahn-testsuite /usr/local/bin/wstest --mode fuzzingclient --spec /config/fuzzingclient.json