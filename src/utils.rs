use std::str::FromStr;

use rss::Channel;

pub fn fetch_episodes(link: &str) -> Result<(), Box<dyn std::error::Error>> {
    match ureq::get(&link).call() {
        Ok(res) => {
            let episodes = res.into_string()?;
            let channel = Channel::from_str(&episodes)?;
            let episodes_vec = channel.items().to_vec();
        }
        Err(e) => {}
    }

    Ok(())
}
