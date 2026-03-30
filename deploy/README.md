# Poiesis service setup

## Install all services
```bash
sudo cp deploy/anky-mind.service /etc/systemd/system/
sudo cp deploy/anky-heart.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable anky-mind anky-heart
sudo systemctl start anky-mind anky-heart
```

## Check Mind slots
```bash
curl http://localhost:8080/slots | python3 -m json.tool
```

## Check via API
```bash
curl https://anky.app/api/v1/mind/status
```

## Check Heart
```bash
curl http://localhost:8188/system_stats
```
