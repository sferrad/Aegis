IMAGE := aegis:0.1.0

build:
	docker build -f confs/Dockerfile -t $(IMAGE) .

run:
	docker run --rm -p 8080:8080 $(IMAGE)

clean:
	docker rmi $(IMAGE)