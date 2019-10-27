#!/bin/bash -ex
D=`mktemp -d`
mkfifo $D/convey

function cleanup {
    echo "cleaning up fifo $D/convey"
    rm $D/convey
    rm $D/result
    rmdir $D
    exit 1
}
trap cleanup EXIT

# spawn this reader. Presumes pmetrics is on the path. Presumes piper
# has connection to psql ready to go via env vars.
pmetrics piper -f "${D}/convey" & # TODO: fix this hanging-around
                                  # process when the script exits.
                                  # now send the piper data
while true; do
    while read URL; do
        echo "Acquiring data from $URL"
        curl $URL > $D/result
        cat $D/result > $D/convey
    done < endpoints
    sleep 5;
done
