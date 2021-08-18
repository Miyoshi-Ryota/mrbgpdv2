#! /bin/bash
docker-compose build --no-cache
docker-compose up &

# dockerコンテナの起動が終わるまで待つ
while ! docker container ls | grep tests_host2 > /dev/null
do
    sleep 1
done

# dockerコンテナ内でコンパイルが終了しプロセスの起動が終わるまで待つ
while ! docker exec tests_host2_1 ps | grep bgp > /dev/null
do
    sleep 1
done

while ! docker exec tests_host1_1 ps | grep bgp > /dev/null
do
    sleep 1
done

# docker execは実行したコマンドの終了コードをそのまま返す。pingは1パケットでも通ったら終了コード0、全部パケロスすると1が返す。
HOST_2_LOOPBACK_IP=10.100.220.3
docker exec tests_host1_1 ping -c 5 $HOST_2_LOOPBACK_IP
TEST_RESULT=$?



docker-compose down
ESC=$(printf '\033')
if [ $TEST_RESULT -eq 0 ]; then
    printf "${ESC}[32m%s${ESC}[m\n" "統合テストが成功しました。"
else
    printf "${ESC}[31m%s${ESC}[m\n" "統合テストが失敗しました。"
fi

exit $TEST_RESULT
