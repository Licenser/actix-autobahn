test-client:
	docker run -it --rm \
		-v "${PWD}/config:/config" \
		-v "${PWD}/reports:/reports" \
		--network host \
		--name fuzzingclient \
		crossbario/autobahn-testsuite /usr/local/bin/wstest --mode fuzzingclient --spec /config/fuzzingclient.json

test-server:
	docker run -it --rm \
		-v "${PWD}/config:/config" \
		-v "${PWD}/reports:/reports" \
		-p 9001:9001 \
		--name fuzzingserver \
		crossbario/autobahn-testsuite /usr/local/bin/wstest --mode fuzzingserver --spec /config/fuzzingserver.json