aoer-wled-doppler
=================
![Build Status](https://github.com/armyofevilrobots/aoer-wled-doppler/actions/workflows/rust.yml/badge.svg)

![](web/doppler-wall-animated.webp)

WLED-doppler provides a `--user` system service which monitors the current time of day
and sets the dimming levels of WLED devices accordingly. It is location configured,
and automatically calculates sunrise/sunset. You can configure the time period over
which the dimming occurs (defaults to 20 minutes). Any WLED devices on the local 
network will be autodiscovered via mDNS, and sane defaults will be used for their
dimming settings. All you should need to do is set your lat/lon in the config, and
the service will do the rest.

There is also an optional audio monitoring subsystem which will look for output on
the configured audio device, and automatically pause/unpause LEDFX depending on
whether there is something playing. 

WLED-doppler can be installed via `cargo install --path .`

The configuration file will be autogenerated at startup if it does not yet exist.
Below is a simple guide to the settings:

```ron
(
    lat: 49.0,   // lat and
    lon: -124.0, // lon values for calculating sunset/sunrise.
    exclusions: [/*"wled-vu-strip._wled._tcp.local."*/], // Don't touch these devices.
    brightnesses: { 
        "wled-vu-strip._wled._tcp.local.": (1, 5), 
        "wled-derek-matrix-1._wled._tcp.local.": (1, 4),
        "wled-derek-desk._wled._tcp.local.": (1, 30),
        // device_name is the key, the (x,y) tuple are min/max brightness values
        // brightness is between 0 and 255, and matches the WLED web config range.
        // If none is supplied, the default will be the current WLED startup default
        // as the maximum, and the minimum will be the maximum divided by 4.0.
    },
    transition_duration: 7200,  // How many seconds to fade to/from the min/max brightness
    loglevel: 3,  // 0: no logging, 1: error, 2: warn, 3: info, 4: debug, 5: TRACE
    logfile: Some("/home/yourname/.wled-doppler/wled-doppler.log"),  // Where to log
    spotify_config: Some(SpotifyConfig(  // Currently not used.
            client_id: "SECRET ID",
            client_secret: "SECRET VALUE"
        )),
    audio_config: Some(AudioConfig(  // Optional audioconfig for monitoring.
            input_device: "default", //iec958:CARD=J380,DEV=0",
            jack: false,  // I've not tested jack integration. YMMV.
        )),
    ledfx_url: Some("http://localhost:8888"), // If set to None, ledfx won't be modified.
    ledfx_idle_cycles: Some(5), // How many 10 second cycles of silence before pausing ledfx 

)

```

SystemD
-------

To install as a SystemD service:

```bash
cp systemd/wled-doppler.service ~/.config/systemd/user/
systemctl --user enable wled-doppler
systemctl --user start wled-doppler
```

*Note:* There is some nominal multi-platform support in this codebase, but it has not been
tested outside of Linux (specifically Ubuntu/PopOS 22.04). I made this thing to scratch my
own itch, but it might be useful for other people with a little elbow grease.
Build automation is coming soon.
