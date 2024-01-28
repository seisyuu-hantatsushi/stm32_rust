# create docker image
```
docker image build -t stm32-rs-1.5.x -f stm32-rs-1.5.x_dockerfile .
```
# enter console at docker image
```
docker run -it -v /home/kaz/work:/home/kaz/work <image id> /bin/bash
```
