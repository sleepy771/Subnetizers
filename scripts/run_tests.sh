#!/bin/sh
trap 'handle_sigint' 1 2 3 6 15

handle_sigint()
{
    echo `jobs -p`
    for proc in `jobs -p`
    do
        echo "Terminating process $proc ..."
        kill ${proc}
        sleep 1
        echo "Process $proc terminated ..."
    done
}


python3 ./receiver.py > ./recv_log.log &
sleep 1
perf stat ../target/release/ipaggregator-rs > ./stats.log 2>./error.log &
sleep 1
python3 ./streamer.py > ./streamer_log.log

