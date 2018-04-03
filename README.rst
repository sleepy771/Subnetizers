# IpAggregator-rs
IpAggregator is utility that, aggregates streamed ip addresses via udp or kafka to ip ranges written in CIDR format.

## Build

.. code-block:: sh
    > git clone git@github.com:sleepy771/Subnetizers.git
    > cd Subnetizers
    > cargo build --release

To run ip aggregator, just put it where you desire and run it. Process attempts to load settings (by default) from two
locations: `/etc/ipaggregator/settings.yaml` or `~/.ipaggregator/settings.yaml`.

.. code-block:: sh
    > mv ./target/release/ipaggregator-rs $TO_WHEREEVER_I_WANT_IT
    > cp ./settings_test.conf.yaml ~/.ipaggregator/settings.yaml
    > vi ~/.ipaggregator/settings.yaml  # To change settings
    > cd $TO_WHEREEVER_I_WANT_IT
    > ./ipaggregator-rs

ipaggregator-rs by default starts udp listener and udp publisher on `localhost:6788` and `localhost:6789` respectively.
You can change this behavior in `settings.yaml` file or using command line arguments.
Listener and publisher can also connect to kafka message broker, but this is untested at the time.

## Explained configuration:

.. code-block:: yaml
    auto_use_zeroed: true  # Automatically add octet_1.octet_2.octet_3.0/32 IPv4 address
    auto_use_broadcast: true  # Automatically adds octet_1.octet_2.octet_3.255/32 IPv4 address
    publish_timer: 30  # How often should be aggregated result streamed.
    receiver:  # listener settings
      receiver: udp  # listener type. Default is udp.
      udp_address: 127.0.0.1:8080  # Socket address where should be udp listener bound. (optional)
      kafka:  # kafka listener settings (optional)
        hosts: [ localhost:9092 ]  # List of kafka bootstrapping hosts.
        topic: ips-in  # Topic form which should listener read.
        group: ips  # Group
    sender:  # publisher settings
      sender: udp  # publisher type.
      udp_address: 127.0.0.1:8081  #  Publisher socket address where will be aggregated ranges sent. (optional)
      kafka:  # kafka publisher settings (optional)
        hosts: [ localhost:9092 ]  # kafka bootstrap hosts
        topic: ips-out  # topic where ipaggregator-rs will send aggregated ip ranges
        ack_duration: 1  # Duration that will aggregator wait for ack.
    log4rs_settings: None  # Path to log4rs config file. (optional)
