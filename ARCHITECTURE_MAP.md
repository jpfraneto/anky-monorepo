# ARCHITECTURE_MAP

Audit date: 2026-03-30  
Repo root: `/home/kithkui/anky`  
Method: source-driven audit of Rust/Axum routes, templates, SQLite schema, local systemd units, and runtime-local services on Poiesis.

## 1. Directory Tree

Notes:
- The raw `find ... | head -500` output is dominated by generated/media assets under `data/images/`, because `find` returns lexical filesystem order rather than a curated source-first view.
- The raw `tree -L 3` output is similarly dominated by persisted runtime artifacts under `data/`.

<details>
<summary>Raw <code>find . -type f ... | head -500</code> output</summary>

```text
./data/images/0243f958-5ab6-436e-a3ab-94cb0179e809.png
./data/images/7be23c93-27c6-4521-a0b0-c7dd7ff8c47b.png
./data/images/1e72bc3a-88c5-4a66-aa62-29e475fe848d.png
./data/images/bee3a191-29a1-4f3f-8b6c-3a44de57aa4e.png
./data/images/1ff39b67-70b9-4a77-93b8-908e17d656e3.png
./data/images/2d439e9f-3763-467a-8ede-88806941d881.png
./data/images/fa21b2e0-6a26-4e31-a654-13f70b65a17f.png
./data/images/1162ddcc-0c6e-4e63-80fe-5c1797641eb2.png
./data/images/101df410-0cec-45f5-8af7-f10e2897c516.png
./data/images/c5fdaf4b-50c1-4f74-9a00-f347ec03d353.png
./data/images/22dc4366-f2ab-44d0-bc96-740ae1ee4d1a.png
./data/images/d8444e7a-3ac9-4752-9b44-c1ee0150943a.png
./data/images/3119c586-9002-437f-b483-ec558b80a7cc.png
./data/images/40ff6d70-0171-4475-8298-b70059acb75c.png
./data/images/f1ef3c43-2614-48e8-9b1e-6c4332d47d5c.png
./data/images/c35daf9a-6ab5-46be-938a-25618987b709.png
./data/images/9e014ec0-b782-452c-9268-5f53a0d7a08a.png
./data/images/5666069c-d519-41f4-8787-0dcc6c17a935.png
./data/images/bc19ccf5-eaa1-4b89-860a-dd8986f897e5.png
./data/images/ecbf45e4-dcf5-47ee-a1dd-4758ca2b8ed5.png
./data/images/38c53516-4bfe-4a3a-9935-9e0aea2cf43d.png
./data/images/prompt_27572679-9a22-42dd-b0de-3e73eb235d32.png
./data/images/3115c06a-a423-4c5a-a2f5-e0aaadc849ab.png
./data/images/81615afe-eec8-48b7-9645-9373d23944d3.png
./data/images/e51e0d1d-f840-4a67-9bfc-b388a07903de.png
./data/images/e0d3c8f6-7250-4b35-bdcd-763c161df666.png
./data/images/91d81602-8f50-49f1-bcc5-a6f04db99d99.png
./data/images/1cb86514-6d69-4b08-b50c-029e4c71aec2.png
./data/images/88e36ac0-3274-4118-8cd7-9b20ac0b7058.png
./data/images/631bc513-801b-4a33-8079-7bd0d978240c.png
./data/images/19000a17-820c-4d8f-933d-5992f30ee0b4.png
./data/images/8adb5a3e-8dd5-4c1b-886d-42a95c229335.png
./data/images/6980b6e2-7355-4ad4-b8d4-5735ac5eb467.png
./data/images/6980b6e2-7355-4ad4-b8d4-5735ac5eb467.webp
./data/images/ff058a6c-79a6-4589-843f-aeecc47bfc3b.png
./data/images/ff058a6c-79a6-4589-843f-aeecc47bfc3b.webp
./data/images/3e221fb8-661e-4aba-a516-fdf1b702eb33.png
./data/images/21570a8c-e75b-4afe-8ab6-c2a87e4dc295.png
./data/images/49e4e05f-35e0-44f4-8332-d26845506f87.png
./data/images/b987cc3e-9e6a-491e-bbd5-8eba99c4d41a.png
./data/images/1208190b-60a1-4235-8cb1-4d541525910d.png
./data/images/ad8b953b-3aff-46e8-91d5-e4ccf4900897.png
./data/images/1e39d32b-d4c0-465b-844b-bef10926450a.png
./data/images/39abc61f-ec39-4093-bb2d-869fdd3de056.png
./data/images/05b7ae07-547a-409b-b11f-9c6b3153e264.png
./data/images/05b7ae07-547a-409b-b11f-9c6b3153e264.webp
./data/images/eda98675-bfe9-42ad-8e61-503534945c86.png
./data/images/eda98675-bfe9-42ad-8e61-503534945c86.webp
./data/images/prompt_1f9fc701-0856-47f5-bf6a-6033a1281aad.png
./data/images/b0ce1111-7175-4423-a8b4-1140bc84d9a5.png
./data/images/b0ce1111-7175-4423-a8b4-1140bc84d9a5.webp
./data/images/f215265f-c9f0-4884-9b38-ebb4fa39dda0.png
./data/images/f215265f-c9f0-4884-9b38-ebb4fa39dda0.webp
./data/images/72cf09cc-4ed7-4e17-a362-4828617398fb.png
./data/images/72cf09cc-4ed7-4e17-a362-4828617398fb.webp
./data/images/d3d3629a-ae38-4292-a8a3-99adeda89cb5.png
./data/images/d3d3629a-ae38-4292-a8a3-99adeda89cb5.webp
./data/images/cd0a098a-2684-44bc-9343-486297d46b2e.png
./data/images/cd0a098a-2684-44bc-9343-486297d46b2e.webp
./data/images/a390c666-f5b3-4479-9896-62930306ebf9.png
./data/images/f870564a-cfe9-4d6a-83c0-73dec7f34e4f.png
./data/images/d19a78b0-237f-4768-a794-4612b8d3e907.png
./data/images/d19a78b0-237f-4768-a794-4612b8d3e907.webp
./data/images/3a46ba7b-0d13-4440-92b3-bb90b1eef8e0.png
./data/images/3a46ba7b-0d13-4440-92b3-bb90b1eef8e0.webp
./data/images/42dbc03d-43b8-4a39-9a60-1c74041d4c37.png
./data/images/42dbc03d-43b8-4a39-9a60-1c74041d4c37.webp
./data/images/96d744b2-835f-4eaf-b45e-5de7cc80a407.png
./data/images/96d744b2-835f-4eaf-b45e-5de7cc80a407.webp
./data/images/7dd4e67e-861c-46ec-a16b-79a65af7c08c.png
./data/images/7dd4e67e-861c-46ec-a16b-79a65af7c08c.webp
./data/images/02b3b56b-ff82-4a28-8aa7-be8a014aa705.png
./data/images/02b3b56b-ff82-4a28-8aa7-be8a014aa705.webp
./data/images/06cdf0ff-52f9-4f29-9d18-36e2502744e2.png
./data/images/06cdf0ff-52f9-4f29-9d18-36e2502744e2.webp
./data/images/1e31b141-2b47-469c-8c17-69dd24d51cc8.png
./data/images/1e31b141-2b47-469c-8c17-69dd24d51cc8.webp
./data/images/688a8669-5bf8-4b78-8dd0-0044ae7ee0f7.png
./data/images/688a8669-5bf8-4b78-8dd0-0044ae7ee0f7.webp
./data/images/ad9101a0-c982-44fd-811d-54b4dd9b1f41.png
./data/images/ad9101a0-c982-44fd-811d-54b4dd9b1f41.webp
./data/images/10f78f0a-fef1-424a-aac7-2706570caebd.png
./data/images/10f78f0a-fef1-424a-aac7-2706570caebd.webp
./data/images/993c2fcb-a77f-4031-9c13-dc6fb20deeee.png
./data/images/993c2fcb-a77f-4031-9c13-dc6fb20deeee.webp
./data/images/b535cfc0-dd5a-4e3e-8fab-899d0ed1cf93.png
./data/images/b535cfc0-dd5a-4e3e-8fab-899d0ed1cf93.webp
./data/images/81d869d4-22f4-4954-a249-b1c53a060d4d.png
./data/images/81d869d4-22f4-4954-a249-b1c53a060d4d.webp
./data/images/ea943b23-b42c-4f02-925a-2d847e8e840f.png
./data/images/ea943b23-b42c-4f02-925a-2d847e8e840f.webp
./data/images/89d34122-653b-4287-b0c2-e7ff1bb3d6f6.png
./data/images/89d34122-653b-4287-b0c2-e7ff1bb3d6f6.webp
./data/images/9a75dbb5-3b0a-4a93-bcb3-49bf150c0981.png
./data/images/9a75dbb5-3b0a-4a93-bcb3-49bf150c0981.webp
./data/images/99d566a9-b8a9-4c29-aca4-bfdad2511329.png
./data/images/99d566a9-b8a9-4c29-aca4-bfdad2511329.webp
./data/images/52013232-f524-4fd1-a1b4-b5a010f27db5.png
./data/images/52013232-f524-4fd1-a1b4-b5a010f27db5.webp
./data/images/ebce7458-d1eb-44a9-969c-0fd88b784afb.png
./data/images/ebce7458-d1eb-44a9-969c-0fd88b784afb.webp
./data/images/5a365052-b5cb-4fc3-9f78-a7ef01a3cf88.png
./data/images/5a365052-b5cb-4fc3-9f78-a7ef01a3cf88.webp
./data/images/3b85dd3e-bf85-41f3-924b-c2ad84a9b450.png
./data/images/3b85dd3e-bf85-41f3-924b-c2ad84a9b450.webp
./data/images/9f2b7ead-ddf3-4943-867a-e465278ecb86.png
./data/images/9f2b7ead-ddf3-4943-867a-e465278ecb86.webp
./data/images/2a928790-ee17-4f92-bbad-e4dfb4ba786d.png
./data/images/2a928790-ee17-4f92-bbad-e4dfb4ba786d.webp
./data/images/05a561c7-a0da-45d9-95bd-17c7a6c60bb2.png
./data/images/05a561c7-a0da-45d9-95bd-17c7a6c60bb2.webp
./data/images/6b72266e-5b60-40e2-acf2-fdb0a1f0f43b.png
./data/images/6b72266e-5b60-40e2-acf2-fdb0a1f0f43b.webp
./data/images/fc85f235-8378-423b-ae19-b0140396c969.png
./data/images/fc85f235-8378-423b-ae19-b0140396c969.webp
./data/images/02c99c6f-92ef-4a44-8f10-62d54d817096.png
./data/images/02c99c6f-92ef-4a44-8f10-62d54d817096.webp
./data/images/0f3b3b58-beea-4d39-9da6-db3ef0a043f8.png
./data/images/0f3b3b58-beea-4d39-9da6-db3ef0a043f8.webp
./data/images/video_af830b6b_00.png
./data/images/video_af830b6b_01.png
./data/images/video_af830b6b_02.png
./data/images/video_af830b6b_03.png
./data/images/video_af830b6b_04.png
./data/images/video_af830b6b_05.png
./data/images/video_af830b6b_06.png
./data/images/video_af830b6b_07.png
./data/images/video_af830b6b_08.png
./data/images/video_af830b6b_09.png
./data/images/video_af830b6b_10.png
./data/images/video_af830b6b_11.png
./data/images/video_af830b6b_12.png
./data/images/video_af830b6b_13.png
./data/images/video_6ccbc2d8_00.png
./data/images/video_6ccbc2d8_02.png
./data/images/video_6ccbc2d8_01.png
./data/images/video_6ccbc2d8_03.png
./data/images/video_6ccbc2d8_04.png
./data/images/video_6ccbc2d8_05.png
./data/images/video_6ccbc2d8_06.png
./data/images/video_6ccbc2d8_07.png
./data/images/video_6ccbc2d8_08.png
./data/images/video_6ccbc2d8_09.png
./data/images/video_6ccbc2d8_11.png
./data/images/video_6ccbc2d8_10.png
./data/images/38632eda-bfea-44d5-b349-b7118c7401a8.png
./data/images/38632eda-bfea-44d5-b349-b7118c7401a8.webp
./data/images/video_20bfada3_00.png
./data/images/video_20bfada3_01.png
./data/images/video_20bfada3_02.png
./data/images/video_20bfada3_03.png
./data/images/video_20bfada3_04.png
./data/images/video_20bfada3_05.png
./data/images/video_20bfada3_06.png
./data/images/video_20bfada3_07.png
./data/images/video_20bfada3_08.png
./data/images/07d0097d-83c4-4055-a089-cd1509073293.png
./data/images/07d0097d-83c4-4055-a089-cd1509073293.webp
./data/images/video_e992abb7_01.png
./data/images/video_e992abb7_00.png
./data/images/video_e992abb7_02.png
./data/images/video_e992abb7_05.png
./data/images/video_e992abb7_03.png
./data/images/video_e992abb7_04.png
./data/images/video_e992abb7_07.png
./data/images/video_e992abb7_08.png
./data/images/video_e992abb7_06.png
./data/images/81572138-55f9-467d-880c-6a62cfb3a0bd.png
./data/images/81572138-55f9-467d-880c-6a62cfb3a0bd.webp
./data/images/81572138-55f9-467d-880c-6a62cfb3a0bd_thumb.webp
./data/images/video_5557ff52_02.png
./data/images/video_5557ff52_01.png
./data/images/video_5557ff52_00.png
./data/images/video_5557ff52_03.png
./data/images/video_5557ff52_04.png
./data/images/video_5557ff52_05.png
./data/images/video_5557ff52_06.png
./data/images/video_5557ff52_07.png
./data/images/d44355bc-9db3-432d-86a3-f6b6aad65923.png
./data/images/d44355bc-9db3-432d-86a3-f6b6aad65923.webp
./data/images/d44355bc-9db3-432d-86a3-f6b6aad65923_thumb.webp
./data/images/8aad5d15-d4e0-4a94-8e46-2f741a941080.png
./data/images/8aad5d15-d4e0-4a94-8e46-2f741a941080.webp
./data/images/8aad5d15-d4e0-4a94-8e46-2f741a941080_thumb.webp
./data/images/8c3eba00-78ac-4f8d-aa36-d779565a9128.png
./data/images/8c3eba00-78ac-4f8d-aa36-d779565a9128.webp
./data/images/8c3eba00-78ac-4f8d-aa36-d779565a9128_thumb.webp
./data/images/6ee01240-e331-4418-a55c-b79ca468182a.png
./data/images/6ee01240-e331-4418-a55c-b79ca468182a.webp
./data/images/6ee01240-e331-4418-a55c-b79ca468182a_thumb.webp
./data/images/9e3cfdf6-57f0-41f1-859e-e2db3a871bce.png
./data/images/9e3cfdf6-57f0-41f1-859e-e2db3a871bce.webp
./data/images/9e3cfdf6-57f0-41f1-859e-e2db3a871bce_thumb.webp
./data/images/35e29c9d-fd89-40d1-813e-49d02dbb8c90.png
./data/images/35e29c9d-fd89-40d1-813e-49d02dbb8c90.webp
./data/images/35e29c9d-fd89-40d1-813e-49d02dbb8c90_thumb.webp
./data/images/6e3506f0-388a-4b79-bb30-1aae0a735816.png
./data/images/6e3506f0-388a-4b79-bb30-1aae0a735816.webp
./data/images/6e3506f0-388a-4b79-bb30-1aae0a735816_thumb.webp
./data/images/18f7f9ba-1b3e-4b18-9d54-1b1e7fc3fca1.png
./data/images/18f7f9ba-1b3e-4b18-9d54-1b1e7fc3fca1.webp
./data/images/18f7f9ba-1b3e-4b18-9d54-1b1e7fc3fca1_thumb.webp
./data/images/video_8bfae113_01.png
./data/images/video_8bfae113_02.png
./data/images/video_8bfae113_00.png
./data/images/video_8bfae113_03.png
./data/images/video_8bfae113_05.png
./data/images/video_8bfae113_04.png
./data/images/video_8bfae113_06.png
./data/images/video_8bfae113_07.png
./data/images/video_8bfae113_09.png
./data/images/video_8bfae113_08.png
./data/images/f3ebab9c-de3e-45dc-9bae-9888ae4ef8fb.png
./data/images/f3ebab9c-de3e-45dc-9bae-9888ae4ef8fb.webp
./data/images/f3ebab9c-de3e-45dc-9bae-9888ae4ef8fb_thumb.webp
./data/images/3dd653cc-0e1d-4d60-9508-6b44b7052864.png
./data/images/3dd653cc-0e1d-4d60-9508-6b44b7052864.webp
./data/images/3dd653cc-0e1d-4d60-9508-6b44b7052864_thumb.webp
./data/images/36831230-20d4-4a21-a999-152c61feb268.png
./data/images/36831230-20d4-4a21-a999-152c61feb268.webp
./data/images/36831230-20d4-4a21-a999-152c61feb268_thumb.webp
./data/images/36415621-940c-4010-8f34-aff5aa012d42.png
./data/images/36415621-940c-4010-8f34-aff5aa012d42.webp
./data/images/36415621-940c-4010-8f34-aff5aa012d42_thumb.webp
./data/images/a7bab964-5f2c-4f05-8b52-d5067700e00a.png
./data/images/a7bab964-5f2c-4f05-8b52-d5067700e00a.webp
./data/images/a7bab964-5f2c-4f05-8b52-d5067700e00a_thumb.webp
./data/images/video_0f341deb_00.png
./data/images/video_0f341deb_01.png
./data/images/video_0f341deb_02.png
./data/images/video_0f341deb_03.png
./data/images/video_0f341deb_04.png
./data/images/video_0f341deb_05.png
./data/images/video_0f341deb_06.png
./data/images/video_0f341deb_07.png
./data/images/video_0f341deb_08.png
./data/images/video_0f341deb_09.png
./data/images/video_0f341deb_10.png
./data/images/video_0f341deb_11.png
./data/images/08817708-b105-4f4b-8587-0c223ec78817.png
./data/images/08817708-b105-4f4b-8587-0c223ec78817.webp
./data/images/08817708-b105-4f4b-8587-0c223ec78817_thumb.webp
./data/images/video_e8985306_00.png
./data/images/video_e8985306_01.png
./data/images/video_e8985306_02.png
./data/images/video_e8985306_03.png
./data/images/video_e8985306_04.png
./data/images/video_e8985306_05.png
./data/images/video_e8985306_06.png
./data/images/video_e8985306_07.png
./data/images/video_e8985306_08.png
./data/images/video_e8985306_09.png
./data/images/video_e8985306_10.png
./data/images/video_e8985306_11.png
./data/images/e605fac1-21d2-4313-bce8-1e5c8c9cb6fc.png
./data/images/e605fac1-21d2-4313-bce8-1e5c8c9cb6fc.webp
./data/images/e605fac1-21d2-4313-bce8-1e5c8c9cb6fc_thumb.webp
./data/images/3d367d94-8f47-47a9-99cf-e9ce7bf6069c.png
./data/images/3d367d94-8f47-47a9-99cf-e9ce7bf6069c.webp
./data/images/3d367d94-8f47-47a9-99cf-e9ce7bf6069c_thumb.webp
./data/images/video_ba9eab3c_00.png
./data/images/video_ba9eab3c_01.png
./data/images/video_ba9eab3c_02.png
./data/images/video_ba9eab3c_03.png
./data/images/video_ba9eab3c_04.png
./data/images/video_ba9eab3c_05.png
./data/images/video_ba9eab3c_06.png
./data/images/video_ba9eab3c_07.png
./data/images/video_ba9eab3c_08.png
./data/images/video_ba9eab3c_09.png
./data/images/video_ba9eab3c_10.png
./data/images/video_ba9eab3c_11.png
./data/images/e425dbc5-6873-40af-b46f-dc5d13cbfbc0.png
./data/images/e425dbc5-6873-40af-b46f-dc5d13cbfbc0.webp
./data/images/e425dbc5-6873-40af-b46f-dc5d13cbfbc0_thumb.webp
./data/images/fdec320d-0d5d-4b6f-8fc9-7120a9d6ef45.png
./data/images/fdec320d-0d5d-4b6f-8fc9-7120a9d6ef45.webp
./data/images/fdec320d-0d5d-4b6f-8fc9-7120a9d6ef45_thumb.webp
./data/images/7c7ad5fc-41d3-4864-93f9-e459a97d041a.png
./data/images/7c7ad5fc-41d3-4864-93f9-e459a97d041a.webp
./data/images/7c7ad5fc-41d3-4864-93f9-e459a97d041a_thumb.webp
./data/images/dccc3619-9c8a-4250-8523-9b9642538a12.png
./data/images/dccc3619-9c8a-4250-8523-9b9642538a12.webp
./data/images/dccc3619-9c8a-4250-8523-9b9642538a12_thumb.webp
./data/images/083cfe55-81a0-48d1-bd58-e49cb900f634.png
./data/images/083cfe55-81a0-48d1-bd58-e49cb900f634.webp
./data/images/083cfe55-81a0-48d1-bd58-e49cb900f634_thumb.webp
./data/images/video_1e65a5cb_00.png
./data/images/video_1e65a5cb_01.png
./data/images/video_1e65a5cb_02.png
./data/images/video_1e65a5cb_03.png
./data/images/video_1e65a5cb_04.png
./data/images/video_1e65a5cb_05.png
./data/images/video_1e65a5cb_06.png
./data/images/video_1e65a5cb_07.png
./data/images/video_1e65a5cb_08.png
./data/images/video_1e65a5cb_09.png
./data/images/video_1e65a5cb_10.png
./data/images/f48d194e-1777-40b0-8a74-cb6770783bc3.png
./data/images/f48d194e-1777-40b0-8a74-cb6770783bc3.webp
./data/images/f48d194e-1777-40b0-8a74-cb6770783bc3_thumb.webp
./data/images/video_1a5e4365_00.png
./data/images/video_1a5e4365_01.png
./data/images/video_1a5e4365_02.png
./data/images/video_1a5e4365_03.png
./data/images/video_1a5e4365_04.png
./data/images/video_1a5e4365_05.png
./data/images/video_1a5e4365_06.png
./data/images/video_1a5e4365_07.png
./data/images/video_1a5e4365_08.png
./data/images/video_1a5e4365_09.png
./data/images/e9db46ee-16e6-44de-ae56-0faed7d54154.png
./data/images/e9db46ee-16e6-44de-ae56-0faed7d54154.webp
./data/images/e9db46ee-16e6-44de-ae56-0faed7d54154_thumb.webp
./data/images/93fa35e9-e4d7-4eb2-84fd-a5f84820d62d.png
./data/images/93fa35e9-e4d7-4eb2-84fd-a5f84820d62d.webp
./data/images/93fa35e9-e4d7-4eb2-84fd-a5f84820d62d_thumb.webp
./data/images/1bb9ceb1-ea24-44b1-9742-4b6696ffef8d.png
./data/images/1bb9ceb1-ea24-44b1-9742-4b6696ffef8d.webp
./data/images/1bb9ceb1-ea24-44b1-9742-4b6696ffef8d_thumb.webp
./data/images/d4ac7a20-9dc2-4af0-93d0-f2470fb92a39.png
./data/images/d4ac7a20-9dc2-4af0-93d0-f2470fb92a39.webp
./data/images/d4ac7a20-9dc2-4af0-93d0-f2470fb92a39_thumb.webp
./data/images/ee6e74ab-4815-4c3c-b280-76b14aa2e060.png
./data/images/ee6e74ab-4815-4c3c-b280-76b14aa2e060.webp
./data/images/ee6e74ab-4815-4c3c-b280-76b14aa2e060_thumb.webp
./data/images/d6eda782-5fe3-48e6-98be-8d3be42e3e85.png
./data/images/d6eda782-5fe3-48e6-98be-8d3be42e3e85.webp
./data/images/d6eda782-5fe3-48e6-98be-8d3be42e3e85_thumb.webp
./data/images/ceed3936-0b60-4e90-88e3-f804e7e34e02.png
./data/images/ceed3936-0b60-4e90-88e3-f804e7e34e02.webp
./data/images/ceed3936-0b60-4e90-88e3-f804e7e34e02_thumb.webp
./data/images/16aac48e-5fad-4446-9a09-c62af14410bf.png
./data/images/16aac48e-5fad-4446-9a09-c62af14410bf.webp
./data/images/16aac48e-5fad-4446-9a09-c62af14410bf_thumb.webp
./data/images/d42e3956-1c48-4d15-bc8f-c231eef27acd.png
./data/images/d42e3956-1c48-4d15-bc8f-c231eef27acd.webp
./data/images/d42e3956-1c48-4d15-bc8f-c231eef27acd_thumb.webp
./data/images/98064017-92c6-4394-9648-1f7ced4b1e4f.png
./data/images/98064017-92c6-4394-9648-1f7ced4b1e4f.webp
./data/images/98064017-92c6-4394-9648-1f7ced4b1e4f_thumb.webp
./data/images/634d52f9-cb8a-4536-8ba4-fb786778a6dd.png
./data/images/634d52f9-cb8a-4536-8ba4-fb786778a6dd.webp
./data/images/634d52f9-cb8a-4536-8ba4-fb786778a6dd_thumb.webp
./data/images/0a72cb6c-cce5-485b-bba5-a87eaa3d02d8.png
./data/images/0a72cb6c-cce5-485b-bba5-a87eaa3d02d8.webp
./data/images/0a72cb6c-cce5-485b-bba5-a87eaa3d02d8_thumb.webp
./data/images/a6f4b534-17f3-4a63-b499-aaecdf2cac78.png
./data/images/a6f4b534-17f3-4a63-b499-aaecdf2cac78.webp
./data/images/a6f4b534-17f3-4a63-b499-aaecdf2cac78_thumb.webp
./data/images/landing_gifs/test_scene_00.gif
./data/images/landing_gifs/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_09.gif
./data/images/landing_gifs/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_08.gif
./data/images/landing_gifs/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_07.gif
./data/images/landing_gifs/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_05.gif
./data/images/landing_gifs/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_06.gif
./data/images/landing_gifs/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_04.gif
./data/images/landing_gifs/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_03.gif
./data/images/landing_gifs/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_02.gif
./data/images/landing_gifs/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_01.gif
./data/images/landing_gifs/1a5e4365-7238-406c-8c78-488ee472b1f3__scene_00.gif
./data/images/landing_gifs/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_10.gif
./data/images/landing_gifs/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_09.gif
./data/images/landing_gifs/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_08.gif
./data/images/landing_gifs/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_07.gif
./data/images/landing_gifs/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_06.gif
./data/images/landing_gifs/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_05.gif
./data/images/landing_gifs/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_03.gif
./data/images/landing_gifs/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_04.gif
./data/images/landing_gifs/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_02.gif
./data/images/landing_gifs/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_01.gif
./data/images/landing_gifs/1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_00.gif
./data/images/landing_gifs/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_10.gif
./data/images/landing_gifs/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_09.gif
./data/images/landing_gifs/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_08.gif
./data/images/landing_gifs/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_07.gif
./data/images/landing_gifs/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_06.gif
./data/images/landing_gifs/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_05.gif
./data/images/landing_gifs/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_04.gif
./data/images/landing_gifs/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_02.gif
./data/images/landing_gifs/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_03.gif
./data/images/landing_gifs/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_01.gif
./data/images/landing_gifs/ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_00.gif
./data/images/landing_gifs/e8985306-28e5-4655-b73e-e2d12c46837b__scene_11.gif
./data/images/landing_gifs/e8985306-28e5-4655-b73e-e2d12c46837b__scene_10.gif
./data/images/landing_gifs/e8985306-28e5-4655-b73e-e2d12c46837b__scene_09.gif
./data/images/landing_gifs/e8985306-28e5-4655-b73e-e2d12c46837b__scene_08.gif
./data/images/landing_gifs/e8985306-28e5-4655-b73e-e2d12c46837b__scene_07.gif
./data/images/landing_gifs/e8985306-28e5-4655-b73e-e2d12c46837b__scene_06.gif
./data/images/landing_gifs/e8985306-28e5-4655-b73e-e2d12c46837b__scene_05.gif
./data/images/landing_gifs/e8985306-28e5-4655-b73e-e2d12c46837b__scene_04.gif
./data/images/landing_gifs/e8985306-28e5-4655-b73e-e2d12c46837b__scene_03.gif
./data/images/landing_gifs/e8985306-28e5-4655-b73e-e2d12c46837b__scene_02.gif
./data/images/landing_gifs/e8985306-28e5-4655-b73e-e2d12c46837b__scene_01.gif
./data/images/landing_gifs/e8985306-28e5-4655-b73e-e2d12c46837b__scene_00.gif
./data/images/landing_gifs/0f341deb-43c4-4285-9234-dcdeded40833__scene_11.gif
./data/images/landing_gifs/0f341deb-43c4-4285-9234-dcdeded40833__scene_10.gif
./data/images/landing_gifs/0f341deb-43c4-4285-9234-dcdeded40833__scene_09.gif
./data/images/landing_gifs/0f341deb-43c4-4285-9234-dcdeded40833__scene_08.gif
./data/images/landing_gifs/0f341deb-43c4-4285-9234-dcdeded40833__scene_07.gif
./data/images/3c069ab9-d14f-427b-8733-b0e6485adf61.png
./data/images/3c069ab9-d14f-427b-8733-b0e6485adf61.webp
./data/images/3c069ab9-d14f-427b-8733-b0e6485adf61_thumb.webp
./data/images/92446073-50d6-4b3e-b3cf-7365ec75d12a.png
./data/images/92446073-50d6-4b3e-b3cf-7365ec75d12a.webp
./data/images/92446073-50d6-4b3e-b3cf-7365ec75d12a_thumb.webp
./data/images/b76a8778-3d33-4510-9bfa-3fc1c5875e2e.png
./data/images/b76a8778-3d33-4510-9bfa-3fc1c5875e2e.webp
./data/images/b76a8778-3d33-4510-9bfa-3fc1c5875e2e_thumb.webp
./data/images/video_60c569da_00.png
./data/images/video_60c569da_01.png
./data/images/video_60c569da_02.png
./data/images/video_60c569da_03.png
./data/images/video_60c569da_04.png
./data/images/video_60c569da_05.png
./data/images/video_60c569da_06.png
./data/images/video_60c569da_07.png
./data/images/video_60c569da_08.png
./data/images/video_60c569da_09.png
./data/images/a74ec392-1c31-412e-9447-389896b82ac5.png
./data/images/a74ec392-1c31-412e-9447-389896b82ac5.webp
./data/images/a74ec392-1c31-412e-9447-389896b82ac5_thumb.webp
./data/images/ac8d34c1-2b83-4e0c-9ccb-467a6913c5c3.png
./data/images/ac8d34c1-2b83-4e0c-9ccb-467a6913c5c3.webp
./data/images/ac8d34c1-2b83-4e0c-9ccb-467a6913c5c3_thumb.webp
./data/images/077f476e-75ee-421f-ac1c-6fe0a9522de2.png
./data/images/077f476e-75ee-421f-ac1c-6fe0a9522de2.webp
./data/images/077f476e-75ee-421f-ac1c-6fe0a9522de2_thumb.webp
./data/images/46707a09-266b-4ac8-9c6b-991531e520df.png
./data/images/46707a09-266b-4ac8-9c6b-991531e520df.webp
./data/images/46707a09-266b-4ac8-9c6b-991531e520df_thumb.webp
./data/images/fefebad8-3ae5-4ba0-981a-949eca820456.png
./data/images/fefebad8-3ae5-4ba0-981a-949eca820456.webp
./data/images/fefebad8-3ae5-4ba0-981a-949eca820456_thumb.webp
./data/images/8ca89491-16ea-42c0-a8ed-8c0701701082.png
./data/images/8ca89491-16ea-42c0-a8ed-8c0701701082.webp
./data/images/8ca89491-16ea-42c0-a8ed-8c0701701082_thumb.webp
./data/images/b3e38269-848c-46b8-ac86-8911c364a42e.png
./data/images/b3e38269-848c-46b8-ac86-8911c364a42e.webp
./data/images/b3e38269-848c-46b8-ac86-8911c364a42e_thumb.webp
./data/images/7b08edcd-cdc4-4e6f-ab31-355886350390.png
./data/images/7b08edcd-cdc4-4e6f-ab31-355886350390.webp
./data/images/7b08edcd-cdc4-4e6f-ab31-355886350390_thumb.webp
./data/images/6160693c-33a8-4b8d-99f5-3d88e4cc571e.png
./data/images/6160693c-33a8-4b8d-99f5-3d88e4cc571e.webp
./data/images/6160693c-33a8-4b8d-99f5-3d88e4cc571e_thumb.webp
./data/images/video_47c35852_00.png
./data/images/video_47c35852_01.png
./data/images/video_47c35852_02.png
./data/images/video_47c35852_03.png
./data/images/video_47c35852_04.png
./data/images/video_47c35852_05.png
./data/images/video_47c35852_06.png
./data/images/video_47c35852_07.png
./data/images/video_47c35852_08.png
./data/images/fa345500-4bcd-47cf-822d-4f4feaaf5656.png
./data/images/fa345500-4bcd-47cf-822d-4f4feaaf5656.webp
./data/images/fa345500-4bcd-47cf-822d-4f4feaaf5656_thumb.webp
./data/images/3854a242-f8d6-43f0-8706-efa2d35ba214.png
./data/images/3854a242-f8d6-43f0-8706-efa2d35ba214.webp
./data/images/3854a242-f8d6-43f0-8706-efa2d35ba214_thumb.webp
./data/images/video_acb73b49_00.png
./data/images/video_94e49660_00.png
./data/images/video_94e49660_00.jpg
./data/images/video_94e49660_01.png
./data/images/video_94e49660_01.jpg
./data/images/video_94e49660_02.png
./data/images/video_94e49660_02.jpg
./data/images/video_94e49660_03.png
./data/images/video_94e49660_03.jpg
./data/images/video_94e49660_04.png
./data/images/video_94e49660_04.jpg
./data/images/video_94e49660_05.png
./data/images/video_94e49660_05.jpg
./data/images/video_94e49660_06.png
./data/images/video_94e49660_06.jpg
./data/images/video_94e49660_07.png
./data/images/video_94e49660_07.jpg
./data/images/1a877907-65ec-46ad-a744-1fc1390ee822.png
./data/images/1a877907-65ec-46ad-a744-1fc1390ee822.webp
./data/images/1a877907-65ec-46ad-a744-1fc1390ee822_thumb.webp
./data/images/75ae3b66-488a-4161-ba5d-36b7f57fa24d.png
./data/images/75ae3b66-488a-4161-ba5d-36b7f57fa24d.webp
./data/images/75ae3b66-488a-4161-ba5d-36b7f57fa24d_thumb.webp
./data/images/73319987-5dc2-4346-9aa5-ea06265eed94.png
./data/images/73319987-5dc2-4346-9aa5-ea06265eed94.webp
./data/images/73319987-5dc2-4346-9aa5-ea06265eed94_thumb.webp
./data/images/video_26419b97_00.png
./data/images/video_26419b97_00.jpg
./data/images/video_26419b97_01.png
./data/images/video_26419b97_01.jpg
./data/images/video_26419b97_02.png
./data/images/video_26419b97_02.jpg
./data/images/video_26419b97_03.png
./data/images/video_26419b97_03.jpg
./data/images/video_26419b97_04.png
./data/images/video_26419b97_04.jpg
./data/images/video_26419b97_05.png
./data/images/video_26419b97_05.jpg
./data/images/video_26419b97_06.png
```

</details>

<details>
<summary>Raw <code>tree -L 3 -I 'node_modules|.git|target|.next|dist'</code> output</summary>

```text
.
├── ANKY_SKILL_v7.2.md
├── [ignored Apple private key].p8
├── CLAUDE.md
├── CURRENT_STATE.md
├── Cargo.lock
├── Cargo.toml
├── IOS_PROMPT_POST_WRITING_FLOW.md
├── MANIFESTO.md
├── Makefile
├── PROMPT.md
├── README.md
├── SOUL.md
├── SWIFT_AGENT_BRIEF.md
├── THE_ANKY_MODEL.md
├── UNDERSTANDING_ANKY.md
├── WHITEPAPER.aux
├── WHITEPAPER.log
├── WHITEPAPER.out
├── WHITEPAPER.pdf
├── WHITEPAPER.tex
├── WHITEPAPER.toc
├── agent-skills
│   └── anky
│       ├── SKILL.md
│       ├── agents
│       ├── manifest.json
│       ├── references
│       ├── scripts
│       └── templates
├── anky.db
├── autopost.log
├── contracts
│   └── AnkyMirrors.sol
├── cursor_2.6.21_amd64.deb
├── data
│   ├── aky.db
│   ├── anky-images
│   │   ├── 1d9cf846-74f5-4f06-8b56-ffebb6910909
│   │   ├── 242e0c39-4b01-4260-bb9a-db44fb9dc62d
│   │   ├── 628b2208-dde0-4e89-899a-ed5621ac21d3
│   │   ├── 84861939-bde9-4e51-844f-d675adf6194f
│   │   ├── c04f0e30-087f-4b92-a0d6-83ed259e1a41
│   │   └── d4baf1be-4ed0-4990-802b-6470fc6c5547
│   ├── anky.db
│   ├── anky.db-shm
│   ├── anky.db-wal
│   ├── anky.log
│   ├── create_videos
│   │   ├── bakery-opening-bell.json
│   │   ├── bedroom-journal.json
│   │   ├── bookstore-whisper.json
│   │   ├── boxing-gym-corner.json
│   │   ├── bus-stop-rain.json
│   │   ├── classroom-after-hours.json
│   │   ├── community-garden.json
│   │   ├── dance-studio.json
│   │   ├── diner-listen.json
│   │   ├── empty-apartment-first-night.json
│   │   ├── family-dinner.json
│   │   ├── fire-escape-voicemail.json
│   │   ├── grocery-aisle.json
│   │   ├── haircut-mirror.json
│   │   ├── hospital-corridor.json
│   │   ├── kitchen-tea.json
│   │   ├── laundromat-fold.json
│   │   ├── mechanic-garage.json
│   │   ├── office-stairwell.json
│   │   ├── park-bench-breakup.json
│   │   ├── recording-booth-first-true-take.json
│   │   ├── rooftop-sunrise.json
│   │   ├── seaside-walk.json
│   │   ├── shelter-adoption-moment.json
│   │   ├── sidewalk-chalk.json
│   │   ├── subway-window.json
│   │   ├── thrift-store-new-self.json
│   │   └── wedding-speech-side-room.json
│   ├── exports
│   │   ├── anky-round-two
│   │   ├── anky-round-two.tar.gz
│   │   ├── final-training-dataset-for-round-two
│   │   └── final-training-dataset-for-round-two.tar.gz
│   ├── generated_training
│   │   ├── 2fbb0278-c47d-4358-9304-0e7ab7569736.png
│   │   ├── 2fbb0278-c47d-4358-9304-0e7ab7569736.txt
│   │   ├── 3ac2f5f7-9d52-43c2-b607-3f582eb19abd.png
│   │   ├── 3ac2f5f7-9d52-43c2-b607-3f582eb19abd.txt
│   │   ├── 48b84655-8436-455d-ab94-3ff2a578edea.png
│   │   ├── 48b84655-8436-455d-ab94-3ff2a578edea.txt
│   │   ├── a79144e9-aeb5-4788-8c8f-0c8bb4d36f26.png
│   │   ├── a79144e9-aeb5-4788-8c8f-0c8bb4d36f26.txt
│   │   └── prompts.json
│   ├── generations
│   │   ├── batch-20260303-124141
│   │   ├── batch-20260303-132654
│   │   ├── batch-20260303-132828
│   │   └── batch-20260303-134016
│   ├── images
│   │   ├── 0070622e-3f2b-4a86-8349-5f7c70d1e629.png
│   │   ├── 0070622e-3f2b-4a86-8349-5f7c70d1e629.webp
│   │   ├── 0070622e-3f2b-4a86-8349-5f7c70d1e629_thumb.webp
│   │   ├── 0129a4f0-2d32-4c10-80df-fff03e2690fe.png
│   │   ├── 0129a4f0-2d32-4c10-80df-fff03e2690fe.webp
│   │   ├── 0129a4f0-2d32-4c10-80df-fff03e2690fe_thumb.webp
│   │   ├── 0243f958-5ab6-436e-a3ab-94cb0179e809.png
│   │   ├── 02b3b56b-ff82-4a28-8aa7-be8a014aa705.png
│   │   ├── 02b3b56b-ff82-4a28-8aa7-be8a014aa705.webp
│   │   ├── 02c99c6f-92ef-4a44-8f10-62d54d817096.png
│   │   ├── 02c99c6f-92ef-4a44-8f10-62d54d817096.webp
│   │   ├── 02f5bbd8-ad0b-4975-ba1e-5e51db38f8cc.png
│   │   ├── 02f5bbd8-ad0b-4975-ba1e-5e51db38f8cc.webp
│   │   ├── 02f5bbd8-ad0b-4975-ba1e-5e51db38f8cc_thumb.webp
│   │   ├── 04bad0c0-e332-418a-8d8a-d5e2928d816a.png
│   │   ├── 04bad0c0-e332-418a-8d8a-d5e2928d816a.webp
│   │   ├── 04bad0c0-e332-418a-8d8a-d5e2928d816a_thumb.webp
│   │   ├── 059e2575-e120-43b6-ade3-b09e6301f7fa.png
│   │   ├── 059e2575-e120-43b6-ade3-b09e6301f7fa.webp
│   │   ├── 059e2575-e120-43b6-ade3-b09e6301f7fa_thumb.webp
│   │   ├── 05a561c7-a0da-45d9-95bd-17c7a6c60bb2.png
│   │   ├── 05a561c7-a0da-45d9-95bd-17c7a6c60bb2.webp
│   │   ├── 05b7ae07-547a-409b-b11f-9c6b3153e264.png
│   │   ├── 05b7ae07-547a-409b-b11f-9c6b3153e264.webp
│   │   ├── 06cdf0ff-52f9-4f29-9d18-36e2502744e2.png
│   │   ├── 06cdf0ff-52f9-4f29-9d18-36e2502744e2.webp
│   │   ├── 077f476e-75ee-421f-ac1c-6fe0a9522de2.png
│   │   ├── 077f476e-75ee-421f-ac1c-6fe0a9522de2.webp
│   │   ├── 077f476e-75ee-421f-ac1c-6fe0a9522de2_thumb.webp
│   │   ├── 07d0097d-83c4-4055-a089-cd1509073293.png
│   │   ├── 07d0097d-83c4-4055-a089-cd1509073293.webp
│   │   ├── 083cfe55-81a0-48d1-bd58-e49cb900f634.png
│   │   ├── 083cfe55-81a0-48d1-bd58-e49cb900f634.webp
│   │   ├── 083cfe55-81a0-48d1-bd58-e49cb900f634_thumb.webp
│   │   ├── 084a75b2-4522-482b-87ee-cca646983c82.png
│   │   ├── 084a75b2-4522-482b-87ee-cca646983c82.webp
│   │   ├── 084a75b2-4522-482b-87ee-cca646983c82_thumb.webp
│   │   ├── 08817708-b105-4f4b-8587-0c223ec78817.png
│   │   ├── 08817708-b105-4f4b-8587-0c223ec78817.webp
│   │   ├── 08817708-b105-4f4b-8587-0c223ec78817_thumb.webp
│   │   ├── 0959f1cd-14f3-48cd-bcfa-7b3ea225fc51.png
│   │   ├── 0959f1cd-14f3-48cd-bcfa-7b3ea225fc51.webp
│   │   ├── 0959f1cd-14f3-48cd-bcfa-7b3ea225fc51_thumb.webp
│   │   ├── 097ea056-ad2f-4d6d-923d-6a90d3d52135.png
│   │   ├── 097ea056-ad2f-4d6d-923d-6a90d3d52135.webp
│   │   ├── 097ea056-ad2f-4d6d-923d-6a90d3d52135_thumb.webp
│   │   ├── 09822d0f-4728-4b67-8ada-5bec05785a42.png
│   │   ├── 09822d0f-4728-4b67-8ada-5bec05785a42.webp
│   │   ├── 09822d0f-4728-4b67-8ada-5bec05785a42_thumb.webp
│   │   ├── 0a72cb6c-cce5-485b-bba5-a87eaa3d02d8.png
│   │   ├── 0a72cb6c-cce5-485b-bba5-a87eaa3d02d8.webp
│   │   ├── 0a72cb6c-cce5-485b-bba5-a87eaa3d02d8_thumb.webp
│   │   ├── 0c239d7d-cf35-4d1e-8b0c-8acc7aad41e2.png
│   │   ├── 0c239d7d-cf35-4d1e-8b0c-8acc7aad41e2.webp
│   │   ├── 0c239d7d-cf35-4d1e-8b0c-8acc7aad41e2_thumb.webp
│   │   ├── 0d6d678e-de05-460c-b9a6-72d20473cc2f.png
│   │   ├── 0d6d678e-de05-460c-b9a6-72d20473cc2f.webp
│   │   ├── 0d6d678e-de05-460c-b9a6-72d20473cc2f_thumb.webp
│   │   ├── 0f3b3b58-beea-4d39-9da6-db3ef0a043f8.png
│   │   ├── 0f3b3b58-beea-4d39-9da6-db3ef0a043f8.webp
│   │   ├── 0f49f504-ba78-466c-bdbf-2209499ffb3c.png
│   │   ├── 0f49f504-ba78-466c-bdbf-2209499ffb3c.webp
│   │   ├── 0f49f504-ba78-466c-bdbf-2209499ffb3c_thumb.webp
│   │   ├── 0fede438-4a6b-4c09-be26-b073a10b60ec.png
│   │   ├── 0fede438-4a6b-4c09-be26-b073a10b60ec.webp
│   │   ├── 0fede438-4a6b-4c09-be26-b073a10b60ec_thumb.webp
│   │   ├── 101df410-0cec-45f5-8af7-f10e2897c516.png
│   │   ├── 103d9391-46c2-4a41-863b-0a296f17438c.png
│   │   ├── 103d9391-46c2-4a41-863b-0a296f17438c.webp
│   │   ├── 103d9391-46c2-4a41-863b-0a296f17438c_thumb.webp
│   │   ├── 10f78f0a-fef1-424a-aac7-2706570caebd.png
│   │   ├── 10f78f0a-fef1-424a-aac7-2706570caebd.webp
│   │   ├── 1162ddcc-0c6e-4e63-80fe-5c1797641eb2.png
│   │   ├── 1208190b-60a1-4235-8cb1-4d541525910d.png
│   │   ├── 1208190b-60a1-4235-8cb1-4d541525910d.txt
│   │   ├── 12cc69de-9cb1-4ff0-ac04-a23fd50e02f0.png
│   │   ├── 12cc69de-9cb1-4ff0-ac04-a23fd50e02f0.webp
│   │   ├── 12cc69de-9cb1-4ff0-ac04-a23fd50e02f0_thumb.webp
│   │   ├── 12e0b184-2be6-4975-a4f0-05cf1d430226.png
│   │   ├── 12e0b184-2be6-4975-a4f0-05cf1d430226.webp
│   │   ├── 12e0b184-2be6-4975-a4f0-05cf1d430226_thumb.webp
│   │   ├── 13096861-0fd3-4830-902c-8126a8d24174.png
│   │   ├── 13096861-0fd3-4830-902c-8126a8d24174.webp
│   │   ├── 13096861-0fd3-4830-902c-8126a8d24174_thumb.webp
│   │   ├── 13fbf56e-df21-444b-82aa-0f4a5c170753.png
│   │   ├── 13fbf56e-df21-444b-82aa-0f4a5c170753.webp
│   │   ├── 13fbf56e-df21-444b-82aa-0f4a5c170753_thumb.webp
│   │   ├── 14a2a999-8d73-4162-b32a-b8b4005212ca.png
│   │   ├── 14a2a999-8d73-4162-b32a-b8b4005212ca.webp
│   │   ├── 14a2a999-8d73-4162-b32a-b8b4005212ca_thumb.webp
│   │   ├── 15106028-ecea-4372-a3a1-eb527b2fdc45.png
│   │   ├── 15106028-ecea-4372-a3a1-eb527b2fdc45.webp
│   │   ├── 15106028-ecea-4372-a3a1-eb527b2fdc45_thumb.webp
│   │   ├── 16aac48e-5fad-4446-9a09-c62af14410bf.png
│   │   ├── 16aac48e-5fad-4446-9a09-c62af14410bf.webp
│   │   ├── 16aac48e-5fad-4446-9a09-c62af14410bf_thumb.webp
│   │   ├── 16daacf6-5d83-4f6a-b998-d50e6806cb51.png
│   │   ├── 16daacf6-5d83-4f6a-b998-d50e6806cb51.webp
│   │   ├── 16daacf6-5d83-4f6a-b998-d50e6806cb51_thumb.webp
│   │   ├── 17035f20-18d8-4c86-aa45-b02f82567eae.png
│   │   ├── 17035f20-18d8-4c86-aa45-b02f82567eae.webp
│   │   ├── 17035f20-18d8-4c86-aa45-b02f82567eae_thumb.webp
│   │   ├── 17a7e081-580f-4d9d-8aee-22d8d35b3220.png
│   │   ├── 17a7e081-580f-4d9d-8aee-22d8d35b3220.webp
│   │   ├── 17a7e081-580f-4d9d-8aee-22d8d35b3220_thumb.webp
│   │   ├── 18078d30-a35b-4351-97f1-eab8ca9224f0.png
│   │   ├── 18078d30-a35b-4351-97f1-eab8ca9224f0.webp
│   │   ├── 18078d30-a35b-4351-97f1-eab8ca9224f0_thumb.webp
│   │   ├── 18d29157-190f-4a27-8400-f45d417f0289.png
│   │   ├── 18d29157-190f-4a27-8400-f45d417f0289.webp
│   │   ├── 18d29157-190f-4a27-8400-f45d417f0289_thumb.webp
│   │   ├── 18f7f9ba-1b3e-4b18-9d54-1b1e7fc3fca1.png
│   │   ├── 18f7f9ba-1b3e-4b18-9d54-1b1e7fc3fca1.webp
│   │   ├── 18f7f9ba-1b3e-4b18-9d54-1b1e7fc3fca1_thumb.webp
│   │   ├── 19000a17-820c-4d8f-933d-5992f30ee0b4.png
│   │   ├── 1a877907-65ec-46ad-a744-1fc1390ee822.png
│   │   ├── 1a877907-65ec-46ad-a744-1fc1390ee822.webp
│   │   ├── 1a877907-65ec-46ad-a744-1fc1390ee822_thumb.webp
│   │   ├── 1b56fbb3-9892-4491-987d-c958e3e921cd.png
│   │   ├── 1b56fbb3-9892-4491-987d-c958e3e921cd.webp
│   │   ├── 1b56fbb3-9892-4491-987d-c958e3e921cd_thumb.webp
│   │   ├── 1b76c080-3731-4db6-9040-f2f6c85d590a.png
│   │   ├── 1b76c080-3731-4db6-9040-f2f6c85d590a.webp
│   │   ├── 1b76c080-3731-4db6-9040-f2f6c85d590a_thumb.webp
│   │   ├── 1bb9ceb1-ea24-44b1-9742-4b6696ffef8d.png
│   │   ├── 1bb9ceb1-ea24-44b1-9742-4b6696ffef8d.webp
│   │   ├── 1bb9ceb1-ea24-44b1-9742-4b6696ffef8d_thumb.webp
│   │   ├── 1c171a4f-46e3-4064-981b-393193819157.png
│   │   ├── 1c171a4f-46e3-4064-981b-393193819157.webp
│   │   ├── 1c171a4f-46e3-4064-981b-393193819157_thumb.webp
│   │   ├── 1c474c6e-b975-4ba0-bf8c-507a5c8e7d2e.png
│   │   ├── 1c474c6e-b975-4ba0-bf8c-507a5c8e7d2e.webp
│   │   ├── 1c474c6e-b975-4ba0-bf8c-507a5c8e7d2e_thumb.webp
│   │   ├── 1c8f355d-04ca-4ccb-9125-5ae06b109e12.png
│   │   ├── 1c8f355d-04ca-4ccb-9125-5ae06b109e12.webp
│   │   ├── 1c8f355d-04ca-4ccb-9125-5ae06b109e12_thumb.webp
│   │   ├── 1cb86514-6d69-4b08-b50c-029e4c71aec2.png
│   │   ├── 1d4cb105-f84c-4b07-823d-0b165c7a63ae.png
│   │   ├── 1d4cb105-f84c-4b07-823d-0b165c7a63ae.webp
│   │   ├── 1d4cb105-f84c-4b07-823d-0b165c7a63ae_thumb.webp
│   │   ├── 1e31b141-2b47-469c-8c17-69dd24d51cc8.png
│   │   ├── 1e31b141-2b47-469c-8c17-69dd24d51cc8.webp
│   │   ├── 1e39d32b-d4c0-465b-844b-bef10926450a.png
│   │   ├── 1e39d32b-d4c0-465b-844b-bef10926450a.txt
│   │   ├── 1e72bc3a-88c5-4a66-aa62-29e475fe848d.png
│   │   ├── 1fe02d5b-2290-4f04-be3e-81618d5f43ce.png
│   │   ├── 1fe02d5b-2290-4f04-be3e-81618d5f43ce.webp
│   │   ├── 1fe02d5b-2290-4f04-be3e-81618d5f43ce_thumb.webp
│   │   ├── 1ff39b67-70b9-4a77-93b8-908e17d656e3.png
│   │   ├── 20260310_091533.png
│   │   ├── 207c639e-d59e-441d-afd5-87389ab915b6.png
│   │   ├── 207c639e-d59e-441d-afd5-87389ab915b6.webp
│   │   ├── 207c639e-d59e-441d-afd5-87389ab915b6_thumb.webp
│   │   ├── 20a95f33-377b-469c-a6ec-3bfc33f5a807.png
│   │   ├── 20a95f33-377b-469c-a6ec-3bfc33f5a807.webp
│   │   ├── 20a95f33-377b-469c-a6ec-3bfc33f5a807_thumb.webp
│   │   ├── 20d36fb2-8886-4c62-9f85-08597ee75b07.png
│   │   ├── 20d36fb2-8886-4c62-9f85-08597ee75b07.webp
│   │   ├── 20d36fb2-8886-4c62-9f85-08597ee75b07_thumb.webp
│   │   ├── 2152e63c-2859-4574-9ae1-99ae3519257c.png
│   │   ├── 2152e63c-2859-4574-9ae1-99ae3519257c.webp
│   │   ├── 2152e63c-2859-4574-9ae1-99ae3519257c_thumb.webp
│   │   ├── 21570a8c-e75b-4afe-8ab6-c2a87e4dc295.png
│   │   ├── 21570a8c-e75b-4afe-8ab6-c2a87e4dc295.txt
│   │   ├── 22bd93ba-cf1d-4ca6-aad2-0ca5ff1f4fc6.png
│   │   ├── 22bd93ba-cf1d-4ca6-aad2-0ca5ff1f4fc6.webp
│   │   ├── 22bd93ba-cf1d-4ca6-aad2-0ca5ff1f4fc6_thumb.webp
│   │   ├── 22dc4366-f2ab-44d0-bc96-740ae1ee4d1a.png
│   │   ├── 233ce11f-a616-4acf-ba2a-bdf4ace63ff6.png
│   │   ├── 233ce11f-a616-4acf-ba2a-bdf4ace63ff6.webp
│   │   ├── 233ce11f-a616-4acf-ba2a-bdf4ace63ff6_thumb.webp
│   │   ├── 25a129a3-7f17-4d75-835e-3d7dd69a2071.png
│   │   ├── 25a129a3-7f17-4d75-835e-3d7dd69a2071.webp
│   │   ├── 25a129a3-7f17-4d75-835e-3d7dd69a2071_thumb.webp
│   │   ├── 25af77f1-0312-4baa-9c24-d3e508f2cd29.png
│   │   ├── 25af77f1-0312-4baa-9c24-d3e508f2cd29.webp
│   │   ├── 25af77f1-0312-4baa-9c24-d3e508f2cd29_thumb.webp
│   │   ├── 279b85ad-b4ab-44ab-9243-9eb7da603003.png
│   │   ├── 279b85ad-b4ab-44ab-9243-9eb7da603003.webp
│   │   ├── 279b85ad-b4ab-44ab-9243-9eb7da603003_thumb.webp
│   │   ├── 29dda527-77e5-4dce-9234-3209cf6f19d2.png
│   │   ├── 29dda527-77e5-4dce-9234-3209cf6f19d2.webp
│   │   ├── 29dda527-77e5-4dce-9234-3209cf6f19d2_thumb.webp
│   │   ├── 2a928790-ee17-4f92-bbad-e4dfb4ba786d.png
│   │   ├── 2a928790-ee17-4f92-bbad-e4dfb4ba786d.webp
│   │   ├── 2abd968d-afc6-4e18-aa81-6a24f2a95c34.png
│   │   ├── 2abd968d-afc6-4e18-aa81-6a24f2a95c34.webp
│   │   ├── 2abd968d-afc6-4e18-aa81-6a24f2a95c34_thumb.webp
│   │   ├── 2b215ab8-b9ed-4a5e-9468-46660c632a92.png
│   │   ├── 2b215ab8-b9ed-4a5e-9468-46660c632a92.webp
│   │   ├── 2b215ab8-b9ed-4a5e-9468-46660c632a92_thumb.webp
│   │   ├── 2d155e9a-1f01-4abc-93e5-6233c375af8d.png
│   │   ├── 2d155e9a-1f01-4abc-93e5-6233c375af8d.webp
│   │   ├── 2d155e9a-1f01-4abc-93e5-6233c375af8d_thumb.webp
│   │   ├── 2d439e9f-3763-467a-8ede-88806941d881.png
│   │   ├── 30369eb3-9be3-468b-a25f-e05e0c8c85fa.png
│   │   ├── 30369eb3-9be3-468b-a25f-e05e0c8c85fa.webp
│   │   ├── 30369eb3-9be3-468b-a25f-e05e0c8c85fa_thumb.webp
│   │   ├── 3115c06a-a423-4c5a-a2f5-e0aaadc849ab.png
│   │   ├── 3119c586-9002-437f-b483-ec558b80a7cc.png
│   │   ├── 314daed9-5873-4fe2-bbe3-ea1d181a8a76.png
│   │   ├── 314daed9-5873-4fe2-bbe3-ea1d181a8a76.webp
│   │   ├── 314daed9-5873-4fe2-bbe3-ea1d181a8a76_thumb.webp
│   │   ├── 332fea05-a1cd-4e7b-878c-cc5f4e72685f.png
│   │   ├── 332fea05-a1cd-4e7b-878c-cc5f4e72685f.webp
│   │   ├── 332fea05-a1cd-4e7b-878c-cc5f4e72685f_thumb.webp
│   │   ├── 34308df9-8fd5-4acc-9691-0d471551bc70.png
│   │   ├── 34308df9-8fd5-4acc-9691-0d471551bc70.webp
│   │   ├── 34308df9-8fd5-4acc-9691-0d471551bc70_thumb.webp
│   │   ├── 352968c6-9e1a-4f95-b107-97a9b2e446eb.png
│   │   ├── 352968c6-9e1a-4f95-b107-97a9b2e446eb.webp
│   │   ├── 352968c6-9e1a-4f95-b107-97a9b2e446eb_thumb.webp
│   │   ├── 356ba4aa-dfb6-4376-83cc-b85ab98ca168.png
│   │   ├── 356ba4aa-dfb6-4376-83cc-b85ab98ca168.webp
│   │   ├── 356ba4aa-dfb6-4376-83cc-b85ab98ca168_thumb.webp
│   │   ├── 35e29c9d-fd89-40d1-813e-49d02dbb8c90.png
│   │   ├── 35e29c9d-fd89-40d1-813e-49d02dbb8c90.webp
│   │   ├── 35e29c9d-fd89-40d1-813e-49d02dbb8c90_thumb.webp
│   │   ├── 36415621-940c-4010-8f34-aff5aa012d42.png
│   │   ├── 36415621-940c-4010-8f34-aff5aa012d42.webp
│   │   ├── 36415621-940c-4010-8f34-aff5aa012d42_thumb.webp
│   │   ├── 36831230-20d4-4a21-a999-152c61feb268.png
│   │   ├── 36831230-20d4-4a21-a999-152c61feb268.webp
│   │   ├── 36831230-20d4-4a21-a999-152c61feb268_thumb.webp
│   │   ├── 36e88d61-1a40-47cd-b078-3b1f3fdced24.png
│   │   ├── 36e88d61-1a40-47cd-b078-3b1f3fdced24.webp
│   │   ├── 36e88d61-1a40-47cd-b078-3b1f3fdced24_thumb.webp
│   │   ├── 382f6128-8b98-43ef-82ef-d6877927b2d6.png
│   │   ├── 382f6128-8b98-43ef-82ef-d6877927b2d6.webp
│   │   ├── 382f6128-8b98-43ef-82ef-d6877927b2d6_thumb.webp
│   │   ├── 3854a242-f8d6-43f0-8706-efa2d35ba214.png
│   │   ├── 3854a242-f8d6-43f0-8706-efa2d35ba214.webp
│   │   ├── 3854a242-f8d6-43f0-8706-efa2d35ba214_thumb.webp
│   │   ├── 38632eda-bfea-44d5-b349-b7118c7401a8.png
│   │   ├── 38632eda-bfea-44d5-b349-b7118c7401a8.webp
│   │   ├── 38c53516-4bfe-4a3a-9935-9e0aea2cf43d.png
│   │   ├── 38f0d603-eaf8-4b63-934f-13d50beb235b.png
│   │   ├── 38f0d603-eaf8-4b63-934f-13d50beb235b.webp
│   │   ├── 38f0d603-eaf8-4b63-934f-13d50beb235b_thumb.webp
│   │   ├── 39abc61f-ec39-4093-bb2d-869fdd3de056.png
│   │   ├── 39abc61f-ec39-4093-bb2d-869fdd3de056.txt
│   │   ├── 39e1b3d8-f926-4023-bd9e-c37a28966ff5.png
│   │   ├── 39e1b3d8-f926-4023-bd9e-c37a28966ff5.webp
│   │   ├── 39e1b3d8-f926-4023-bd9e-c37a28966ff5_thumb.webp
│   │   ├── 3a46ba7b-0d13-4440-92b3-bb90b1eef8e0.png
│   │   ├── 3a46ba7b-0d13-4440-92b3-bb90b1eef8e0.webp
│   │   ├── 3b85dd3e-bf85-41f3-924b-c2ad84a9b450.png
│   │   ├── 3b85dd3e-bf85-41f3-924b-c2ad84a9b450.webp
│   │   ├── 3c069ab9-d14f-427b-8733-b0e6485adf61.png
│   │   ├── 3c069ab9-d14f-427b-8733-b0e6485adf61.webp
│   │   ├── 3c069ab9-d14f-427b-8733-b0e6485adf61_thumb.webp
│   │   ├── 3d367d94-8f47-47a9-99cf-e9ce7bf6069c.png
│   │   ├── 3d367d94-8f47-47a9-99cf-e9ce7bf6069c.webp
│   │   ├── 3d367d94-8f47-47a9-99cf-e9ce7bf6069c_thumb.webp
│   │   ├── 3dd653cc-0e1d-4d60-9508-6b44b7052864.png
│   │   ├── 3dd653cc-0e1d-4d60-9508-6b44b7052864.webp
│   │   ├── 3dd653cc-0e1d-4d60-9508-6b44b7052864_thumb.webp
│   │   ├── 3e221fb8-661e-4aba-a516-fdf1b702eb33.png
│   │   ├── 3e221fb8-661e-4aba-a516-fdf1b702eb33.txt
│   │   ├── 3e8e5639-89ac-4d79-89c6-97304f365e9a.png
│   │   ├── 3e8e5639-89ac-4d79-89c6-97304f365e9a.webp
│   │   ├── 3e8e5639-89ac-4d79-89c6-97304f365e9a_thumb.webp
│   │   ├── 3fbb4b62-e622-4833-9be6-66dff220d227.png
│   │   ├── 3fbb4b62-e622-4833-9be6-66dff220d227.webp
│   │   ├── 3fbb4b62-e622-4833-9be6-66dff220d227_thumb.webp
│   │   ├── 3fcf92fe-bd8a-4385-8335-6ed292369e23.png
│   │   ├── 3fcf92fe-bd8a-4385-8335-6ed292369e23.webp
│   │   ├── 3fcf92fe-bd8a-4385-8335-6ed292369e23_thumb.webp
│   │   ├── 3fe60a80-045d-4c36-86d1-75a1a64fcde1.png
│   │   ├── 3fe60a80-045d-4c36-86d1-75a1a64fcde1.webp
│   │   ├── 3fe60a80-045d-4c36-86d1-75a1a64fcde1_thumb.webp
│   │   ├── 40572b92-31e4-4ff6-9b5b-dc7d069194f4.png
│   │   ├── 40572b92-31e4-4ff6-9b5b-dc7d069194f4.webp
│   │   ├── 40572b92-31e4-4ff6-9b5b-dc7d069194f4_thumb.webp
│   │   ├── 40ff6d70-0171-4475-8298-b70059acb75c.png
│   │   ├── 417d8a80-b743-40dd-aa92-77936b168fb4.png
│   │   ├── 417d8a80-b743-40dd-aa92-77936b168fb4.webp
│   │   ├── 417d8a80-b743-40dd-aa92-77936b168fb4_thumb.webp
│   │   ├── 41fab5d8-4453-4958-b7ab-4e61b62fe888.png
│   │   ├── 41fab5d8-4453-4958-b7ab-4e61b62fe888.webp
│   │   ├── 41fab5d8-4453-4958-b7ab-4e61b62fe888_thumb.webp
│   │   ├── 42109938-0ece-4492-8bf4-b417d704c38b.png
│   │   ├── 42109938-0ece-4492-8bf4-b417d704c38b.webp
│   │   ├── 42109938-0ece-4492-8bf4-b417d704c38b_thumb.webp
│   │   ├── 427ed053-3cf1-4112-a151-fd3131d322ce.png
│   │   ├── 427ed053-3cf1-4112-a151-fd3131d322ce.webp
│   │   ├── 427ed053-3cf1-4112-a151-fd3131d322ce_thumb.webp
│   │   ├── 42852da0-0702-40b9-81fc-a183c7665cf7.png
│   │   ├── 42852da0-0702-40b9-81fc-a183c7665cf7.webp
│   │   ├── 42852da0-0702-40b9-81fc-a183c7665cf7_thumb.webp
│   │   ├── 42dbc03d-43b8-4a39-9a60-1c74041d4c37.png
│   │   ├── 42dbc03d-43b8-4a39-9a60-1c74041d4c37.webp
│   │   ├── 46707a09-266b-4ac8-9c6b-991531e520df.png
│   │   ├── 46707a09-266b-4ac8-9c6b-991531e520df.webp
│   │   ├── 46707a09-266b-4ac8-9c6b-991531e520df_thumb.webp
│   │   ├── 47761cac-3fe7-41c0-8f8c-fddd60be9299.png
│   │   ├── 47761cac-3fe7-41c0-8f8c-fddd60be9299.webp
│   │   ├── 47761cac-3fe7-41c0-8f8c-fddd60be9299_thumb.webp
│   │   ├── 483b490b-b8eb-474d-bab8-c1abb64e31d0.png
│   │   ├── 483b490b-b8eb-474d-bab8-c1abb64e31d0.webp
│   │   ├── 483b490b-b8eb-474d-bab8-c1abb64e31d0_thumb.webp
│   │   ├── 4895b05b-9ef3-47cb-8c2c-7c9fa3038095.png
│   │   ├── 4895b05b-9ef3-47cb-8c2c-7c9fa3038095.webp
│   │   ├── 4895b05b-9ef3-47cb-8c2c-7c9fa3038095_thumb.webp
│   │   ├── 494f3019-82c7-4d0f-a681-07e21027e8eb.png
│   │   ├── 494f3019-82c7-4d0f-a681-07e21027e8eb.webp
│   │   ├── 494f3019-82c7-4d0f-a681-07e21027e8eb_thumb.webp
│   │   ├── 49e4e05f-35e0-44f4-8332-d26845506f87.png
│   │   ├── 49e4e05f-35e0-44f4-8332-d26845506f87.txt
│   │   ├── 49ee1b49-ad74-477b-b211-ec0d646b9bd6.png
│   │   ├── 49ee1b49-ad74-477b-b211-ec0d646b9bd6.webp
│   │   ├── 49ee1b49-ad74-477b-b211-ec0d646b9bd6_thumb.webp
│   │   ├── 4d12e147-72f7-4ac1-b03a-91d254423670.png
│   │   ├── 4d12e147-72f7-4ac1-b03a-91d254423670.webp
│   │   ├── 4d12e147-72f7-4ac1-b03a-91d254423670_thumb.webp
│   │   ├── 4f801c10-f4b8-4d53-8a66-a9da56d468eb.png
│   │   ├── 4f801c10-f4b8-4d53-8a66-a9da56d468eb.webp
│   │   ├── 4f801c10-f4b8-4d53-8a66-a9da56d468eb_thumb.webp
│   │   ├── 4f8b83c0-2e1b-4945-8b18-e2e7fccd53cd.png
│   │   ├── 4f8b83c0-2e1b-4945-8b18-e2e7fccd53cd.webp
│   │   ├── 4f8b83c0-2e1b-4945-8b18-e2e7fccd53cd_thumb.webp
│   │   ├── 4fded784-c955-4f78-ad50-73b5bb0c7a39.png
│   │   ├── 4fded784-c955-4f78-ad50-73b5bb0c7a39.webp
│   │   ├── 4fded784-c955-4f78-ad50-73b5bb0c7a39_thumb.webp
│   │   ├── 52013232-f524-4fd1-a1b4-b5a010f27db5.png
│   │   ├── 52013232-f524-4fd1-a1b4-b5a010f27db5.webp
│   │   ├── 5398b14a-6020-483f-a8d7-2e6dbf815dd0.png
│   │   ├── 5398b14a-6020-483f-a8d7-2e6dbf815dd0.webp
│   │   ├── 5398b14a-6020-483f-a8d7-2e6dbf815dd0_thumb.webp
│   │   ├── 551dbc60-1e7b-4e50-955d-6371bca13b28.png
│   │   ├── 551dbc60-1e7b-4e50-955d-6371bca13b28.webp
│   │   ├── 551dbc60-1e7b-4e50-955d-6371bca13b28_thumb.webp
│   │   ├── 5666069c-d519-41f4-8787-0dcc6c17a935.png
│   │   ├── 56fa3d49-f955-4af9-bc2c-843fa59d10a0.png
│   │   ├── 56fa3d49-f955-4af9-bc2c-843fa59d10a0.webp
│   │   ├── 56fa3d49-f955-4af9-bc2c-843fa59d10a0_thumb.webp
│   │   ├── 584d7c3b-6aef-41dc-87bc-ea5aa1d9b6a0.png
│   │   ├── 584d7c3b-6aef-41dc-87bc-ea5aa1d9b6a0.webp
│   │   ├── 584d7c3b-6aef-41dc-87bc-ea5aa1d9b6a0_thumb.webp
│   │   ├── 59f4e4d0-ebe6-4ee6-99bb-fc791daed8bd.png
│   │   ├── 59f4e4d0-ebe6-4ee6-99bb-fc791daed8bd.webp
│   │   ├── 59f4e4d0-ebe6-4ee6-99bb-fc791daed8bd_thumb.webp
│   │   ├── 5a365052-b5cb-4fc3-9f78-a7ef01a3cf88.png
│   │   ├── 5a365052-b5cb-4fc3-9f78-a7ef01a3cf88.webp
│   │   ├── 5ddf986b-5632-441d-ae00-4d50043b1fc9.png
│   │   ├── 5ddf986b-5632-441d-ae00-4d50043b1fc9.webp
│   │   ├── 5ddf986b-5632-441d-ae00-4d50043b1fc9_thumb.webp
│   │   ├── 600dc9a7-b2e7-4d81-b25f-b0eb3c8ffb87.png
│   │   ├── 600dc9a7-b2e7-4d81-b25f-b0eb3c8ffb87.webp
│   │   ├── 600dc9a7-b2e7-4d81-b25f-b0eb3c8ffb87_thumb.webp
│   │   ├── 6160693c-33a8-4b8d-99f5-3d88e4cc571e.png
│   │   ├── 6160693c-33a8-4b8d-99f5-3d88e4cc571e.webp
│   │   ├── 6160693c-33a8-4b8d-99f5-3d88e4cc571e_thumb.webp
│   │   ├── 631bc513-801b-4a33-8079-7bd0d978240c.png
│   │   ├── 631d4666-2bc8-4467-ae6f-656a2384c808.png
│   │   ├── 631d4666-2bc8-4467-ae6f-656a2384c808.webp
│   │   ├── 631d4666-2bc8-4467-ae6f-656a2384c808_thumb.webp
│   │   ├── 634d52f9-cb8a-4536-8ba4-fb786778a6dd.png
│   │   ├── 634d52f9-cb8a-4536-8ba4-fb786778a6dd.webp
│   │   ├── 634d52f9-cb8a-4536-8ba4-fb786778a6dd_thumb.webp
│   │   ├── 63982a73-32af-482c-86ac-7ee586673813.png
│   │   ├── 63982a73-32af-482c-86ac-7ee586673813.webp
│   │   ├── 63982a73-32af-482c-86ac-7ee586673813_thumb.webp
│   │   ├── 64102541-44cf-411d-9a32-3849086d62b6.png
│   │   ├── 64102541-44cf-411d-9a32-3849086d62b6.webp
│   │   ├── 64102541-44cf-411d-9a32-3849086d62b6_thumb.webp
│   │   ├── 64255004-5fc7-4a55-8b4d-1c9927da2343.png
│   │   ├── 64255004-5fc7-4a55-8b4d-1c9927da2343.webp
│   │   ├── 64255004-5fc7-4a55-8b4d-1c9927da2343_thumb.webp
│   │   ├── 643b6cb0-d7ef-4604-8cae-a168828b6011.png
│   │   ├── 643b6cb0-d7ef-4604-8cae-a168828b6011.webp
│   │   ├── 643b6cb0-d7ef-4604-8cae-a168828b6011_thumb.webp
│   │   ├── 664d5b27-1a7c-454e-9517-4aeb524e5bca.png
│   │   ├── 664d5b27-1a7c-454e-9517-4aeb524e5bca.webp
│   │   ├── 664d5b27-1a7c-454e-9517-4aeb524e5bca_thumb.webp
│   │   ├── 670759d5-a898-47f2-9ca4-9b5713b0a26c.png
│   │   ├── 670759d5-a898-47f2-9ca4-9b5713b0a26c.webp
│   │   ├── 670759d5-a898-47f2-9ca4-9b5713b0a26c_thumb.webp
│   │   ├── 67daaf83-7359-4f44-abf3-93fb8ffe8a02.png
│   │   ├── 67daaf83-7359-4f44-abf3-93fb8ffe8a02.webp
│   │   ├── 67daaf83-7359-4f44-abf3-93fb8ffe8a02_thumb.webp
│   │   ├── 68004fb4-d21e-4901-91ee-7e760437402d.png
│   │   ├── 68004fb4-d21e-4901-91ee-7e760437402d.webp
│   │   ├── 68004fb4-d21e-4901-91ee-7e760437402d_thumb.webp
│   │   ├── 68893970-1a1d-4f50-b064-85f04199b9e2.png
│   │   ├── 68893970-1a1d-4f50-b064-85f04199b9e2.webp
│   │   ├── 68893970-1a1d-4f50-b064-85f04199b9e2_thumb.webp
│   │   ├── 688a8669-5bf8-4b78-8dd0-0044ae7ee0f7.png
│   │   ├── 688a8669-5bf8-4b78-8dd0-0044ae7ee0f7.webp
│   │   ├── 6980b6e2-7355-4ad4-b8d4-5735ac5eb467.png
│   │   ├── 6980b6e2-7355-4ad4-b8d4-5735ac5eb467.webp
│   │   ├── 6a204c67-ac5c-431b-a296-8e3d7822f61e.png
│   │   ├── 6a204c67-ac5c-431b-a296-8e3d7822f61e.webp
│   │   ├── 6a204c67-ac5c-431b-a296-8e3d7822f61e_thumb.webp
│   │   ├── 6b3576d1-fcfe-484e-9e3e-ba896d38a612.png
│   │   ├── 6b3576d1-fcfe-484e-9e3e-ba896d38a612.webp
│   │   ├── 6b3576d1-fcfe-484e-9e3e-ba896d38a612_thumb.webp
│   │   ├── 6b363443-a528-44e1-bc76-e2851dfbba77.png
│   │   ├── 6b363443-a528-44e1-bc76-e2851dfbba77.webp
│   │   ├── 6b363443-a528-44e1-bc76-e2851dfbba77_thumb.webp
│   │   ├── 6b72266e-5b60-40e2-acf2-fdb0a1f0f43b.png
│   │   ├── 6b72266e-5b60-40e2-acf2-fdb0a1f0f43b.webp
│   │   ├── 6c50cc2f-c045-48a3-9a98-3d68355f4aea.png
│   │   ├── 6c50cc2f-c045-48a3-9a98-3d68355f4aea.webp
│   │   ├── 6c50cc2f-c045-48a3-9a98-3d68355f4aea_thumb.webp
│   │   ├── 6d8161f6-5dd5-4962-859c-912b73e44247.png
│   │   ├── 6d8161f6-5dd5-4962-859c-912b73e44247.webp
│   │   ├── 6d8161f6-5dd5-4962-859c-912b73e44247_thumb.webp
│   │   ├── 6dcdec6a-e2f4-43c0-a450-6062d4bfd7ea.png
│   │   ├── 6dcdec6a-e2f4-43c0-a450-6062d4bfd7ea.webp
│   │   ├── 6dcdec6a-e2f4-43c0-a450-6062d4bfd7ea_thumb.webp
│   │   ├── 6e3506f0-388a-4b79-bb30-1aae0a735816.png
│   │   ├── 6e3506f0-388a-4b79-bb30-1aae0a735816.webp
│   │   ├── 6e3506f0-388a-4b79-bb30-1aae0a735816_thumb.webp
│   │   ├── 6e60bc3b-3246-4618-824d-bcfb39d2d688.png
│   │   ├── 6e60bc3b-3246-4618-824d-bcfb39d2d688.webp
│   │   ├── 6e60bc3b-3246-4618-824d-bcfb39d2d688_thumb.webp
│   │   ├── 6ee01240-e331-4418-a55c-b79ca468182a.png
│   │   ├── 6ee01240-e331-4418-a55c-b79ca468182a.webp
│   │   ├── 6ee01240-e331-4418-a55c-b79ca468182a_thumb.webp
│   │   ├── 6fba1ae6-f86d-4d14-b4b2-c20739c3589d.png
│   │   ├── 6fba1ae6-f86d-4d14-b4b2-c20739c3589d.webp
│   │   ├── 6fba1ae6-f86d-4d14-b4b2-c20739c3589d_thumb.webp
│   │   ├── 7245d89e-d833-4aa4-afa0-46905e364e8f.png
│   │   ├── 7245d89e-d833-4aa4-afa0-46905e364e8f.webp
│   │   ├── 7245d89e-d833-4aa4-afa0-46905e364e8f_thumb.webp
│   │   ├── 72a11b6e-3a25-451c-977a-8d5c39dd78f0.png
│   │   ├── 72a11b6e-3a25-451c-977a-8d5c39dd78f0.webp
│   │   ├── 72a11b6e-3a25-451c-977a-8d5c39dd78f0_thumb.webp
│   │   ├── 72cf09cc-4ed7-4e17-a362-4828617398fb.png
│   │   ├── 72cf09cc-4ed7-4e17-a362-4828617398fb.webp
│   │   ├── 73319987-5dc2-4346-9aa5-ea06265eed94.png
│   │   ├── 73319987-5dc2-4346-9aa5-ea06265eed94.webp
│   │   ├── 73319987-5dc2-4346-9aa5-ea06265eed94_thumb.webp
│   │   ├── 74e34b42-e4b1-4d9c-b4c2-9ed37ed00528.png
│   │   ├── 74e34b42-e4b1-4d9c-b4c2-9ed37ed00528.webp
│   │   ├── 74e34b42-e4b1-4d9c-b4c2-9ed37ed00528_thumb.webp
│   │   ├── 7553c726-289c-4075-b777-25097d7a4e5c.png
│   │   ├── 7553c726-289c-4075-b777-25097d7a4e5c.webp
│   │   ├── 7553c726-289c-4075-b777-25097d7a4e5c_thumb.webp
│   │   ├── 75ae3b66-488a-4161-ba5d-36b7f57fa24d.png
│   │   ├── 75ae3b66-488a-4161-ba5d-36b7f57fa24d.webp
│   │   ├── 75ae3b66-488a-4161-ba5d-36b7f57fa24d_thumb.webp
│   │   ├── 75cc242b-bf0a-4f84-8519-c5f4a4a220e9.png
│   │   ├── 75cc242b-bf0a-4f84-8519-c5f4a4a220e9.webp
│   │   ├── 75cc242b-bf0a-4f84-8519-c5f4a4a220e9_thumb.webp
│   │   ├── 77d87037-19d8-4f20-9a4c-b2d62afce748.png
│   │   ├── 77d87037-19d8-4f20-9a4c-b2d62afce748.webp
│   │   ├── 77d87037-19d8-4f20-9a4c-b2d62afce748_thumb.webp
│   │   ├── 77e9627f-8d9f-40ff-aac6-10664693760d.png
│   │   ├── 77e9627f-8d9f-40ff-aac6-10664693760d.webp
│   │   ├── 77e9627f-8d9f-40ff-aac6-10664693760d_thumb.webp
│   │   ├── 7b08edcd-cdc4-4e6f-ab31-355886350390.png
│   │   ├── 7b08edcd-cdc4-4e6f-ab31-355886350390.webp
│   │   ├── 7b08edcd-cdc4-4e6f-ab31-355886350390_thumb.webp
│   │   ├── 7be23c93-27c6-4521-a0b0-c7dd7ff8c47b.png
│   │   ├── 7c7ad5fc-41d3-4864-93f9-e459a97d041a.png
│   │   ├── 7c7ad5fc-41d3-4864-93f9-e459a97d041a.webp
│   │   ├── 7c7ad5fc-41d3-4864-93f9-e459a97d041a_thumb.webp
│   │   ├── 7dd4e67e-861c-46ec-a16b-79a65af7c08c.png
│   │   ├── 7dd4e67e-861c-46ec-a16b-79a65af7c08c.webp
│   │   ├── 804f23c2-bf55-456c-9424-a7b542fe9874.png
│   │   ├── 804f23c2-bf55-456c-9424-a7b542fe9874.webp
│   │   ├── 804f23c2-bf55-456c-9424-a7b542fe9874_thumb.webp
│   │   ├── 81572138-55f9-467d-880c-6a62cfb3a0bd.png
│   │   ├── 81572138-55f9-467d-880c-6a62cfb3a0bd.webp
│   │   ├── 81572138-55f9-467d-880c-6a62cfb3a0bd_thumb.webp
│   │   ├── 81615afe-eec8-48b7-9645-9373d23944d3.png
│   │   ├── 818e94cb-46ec-4a8c-8e39-9bd56d2ebb63.png
│   │   ├── 818e94cb-46ec-4a8c-8e39-9bd56d2ebb63.webp
│   │   ├── 818e94cb-46ec-4a8c-8e39-9bd56d2ebb63_thumb.webp
│   │   ├── 81d869d4-22f4-4954-a249-b1c53a060d4d.png
│   │   ├── 81d869d4-22f4-4954-a249-b1c53a060d4d.webp
│   │   ├── 81e19046-e2a5-4e93-a3f2-1a13e7a96c2b.png
│   │   ├── 81e19046-e2a5-4e93-a3f2-1a13e7a96c2b.webp
│   │   ├── 81e19046-e2a5-4e93-a3f2-1a13e7a96c2b_thumb.webp
│   │   ├── 821d5d32-dd04-4eb5-bb4e-b4f8f7bc01c5.png
│   │   ├── 821d5d32-dd04-4eb5-bb4e-b4f8f7bc01c5.webp
│   │   ├── 821d5d32-dd04-4eb5-bb4e-b4f8f7bc01c5_thumb.webp
│   │   ├── 82dc6aa1-3137-4cd3-a699-3fc50a22fbfe.png
│   │   ├── 82dc6aa1-3137-4cd3-a699-3fc50a22fbfe.webp
│   │   ├── 82dc6aa1-3137-4cd3-a699-3fc50a22fbfe_thumb.webp
│   │   ├── 833235d0-55ab-412e-81c4-055657a1b224.png
│   │   ├── 833235d0-55ab-412e-81c4-055657a1b224.webp
│   │   ├── 833235d0-55ab-412e-81c4-055657a1b224_thumb.webp
│   │   ├── 83750753-7320-4225-a02f-9537acc8556a.png
│   │   ├── 83750753-7320-4225-a02f-9537acc8556a.webp
│   │   ├── 83750753-7320-4225-a02f-9537acc8556a_thumb.webp
│   │   ├── 849a6ff6-0a71-43fd-be11-fe5366f914c7.png
│   │   ├── 849a6ff6-0a71-43fd-be11-fe5366f914c7.webp
│   │   ├── 849a6ff6-0a71-43fd-be11-fe5366f914c7_thumb.webp
│   │   ├── 84b710cd-cd22-4e17-86e6-ce605f8663cd.png
│   │   ├── 84b710cd-cd22-4e17-86e6-ce605f8663cd.webp
│   │   ├── 84b710cd-cd22-4e17-86e6-ce605f8663cd_thumb.webp
│   │   ├── 84e85e55-00da-4ce0-862f-f8e2bac00d55.png
│   │   ├── 84e85e55-00da-4ce0-862f-f8e2bac00d55.webp
│   │   ├── 84e85e55-00da-4ce0-862f-f8e2bac00d55_thumb.webp
│   │   ├── 86488edf-1162-4c7b-8234-736c0f802dc8.png
│   │   ├── 86488edf-1162-4c7b-8234-736c0f802dc8.webp
│   │   ├── 86488edf-1162-4c7b-8234-736c0f802dc8_thumb.webp
│   │   ├── 86aa9ff7-95e3-4d46-9ddd-1943d7594089.png
│   │   ├── 86aa9ff7-95e3-4d46-9ddd-1943d7594089.webp
│   │   ├── 86aa9ff7-95e3-4d46-9ddd-1943d7594089_thumb.webp
│   │   ├── 86e87de5-bb76-45a6-bac1-7fe5ed9576e2.png
│   │   ├── 86e87de5-bb76-45a6-bac1-7fe5ed9576e2.webp
│   │   ├── 86e87de5-bb76-45a6-bac1-7fe5ed9576e2_thumb.webp
│   │   ├── 87c932f8-ac95-467f-b91b-355211aa2117.png
│   │   ├── 87c932f8-ac95-467f-b91b-355211aa2117.webp
│   │   ├── 87c932f8-ac95-467f-b91b-355211aa2117_thumb.webp
│   │   ├── 88e36ac0-3274-4118-8cd7-9b20ac0b7058.png
│   │   ├── 894fadeb-d0bf-4927-8c13-1882b8c9c246.png
│   │   ├── 894fadeb-d0bf-4927-8c13-1882b8c9c246.webp
│   │   ├── 894fadeb-d0bf-4927-8c13-1882b8c9c246_thumb.webp
│   │   ├── 89cfa723-7cbc-49a5-a392-a15ce7025506.png
│   │   ├── 89cfa723-7cbc-49a5-a392-a15ce7025506.webp
│   │   ├── 89cfa723-7cbc-49a5-a392-a15ce7025506_thumb.webp
│   │   ├── 89d34122-653b-4287-b0c2-e7ff1bb3d6f6.png
│   │   ├── 89d34122-653b-4287-b0c2-e7ff1bb3d6f6.webp
│   │   ├── 8aad5d15-d4e0-4a94-8e46-2f741a941080.png
│   │   ├── 8aad5d15-d4e0-4a94-8e46-2f741a941080.webp
│   │   ├── 8aad5d15-d4e0-4a94-8e46-2f741a941080_thumb.webp
│   │   ├── 8adb5a3e-8dd5-4c1b-886d-42a95c229335.png
│   │   ├── 8b039654-6d1f-403a-84a5-5313160c53d2.png
│   │   ├── 8b039654-6d1f-403a-84a5-5313160c53d2.webp
│   │   ├── 8b039654-6d1f-403a-84a5-5313160c53d2_thumb.webp
│   │   ├── 8be4cb90-e352-4fa7-83f2-1946f88f8187.png
│   │   ├── 8be4cb90-e352-4fa7-83f2-1946f88f8187.webp
│   │   ├── 8be4cb90-e352-4fa7-83f2-1946f88f8187_thumb.webp
│   │   ├── 8c3eba00-78ac-4f8d-aa36-d779565a9128.png
│   │   ├── 8c3eba00-78ac-4f8d-aa36-d779565a9128.webp
│   │   ├── 8c3eba00-78ac-4f8d-aa36-d779565a9128_thumb.webp
│   │   ├── 8ca89491-16ea-42c0-a8ed-8c0701701082.png
│   │   ├── 8ca89491-16ea-42c0-a8ed-8c0701701082.webp
│   │   ├── 8ca89491-16ea-42c0-a8ed-8c0701701082_thumb.webp
│   │   ├── 8d49ffe9-616b-4b50-81cd-5e049d11db52.png
│   │   ├── 8d49ffe9-616b-4b50-81cd-5e049d11db52.webp
│   │   ├── 8d49ffe9-616b-4b50-81cd-5e049d11db52_thumb.webp
│   │   ├── 8d5053aa-03fc-4f3d-ae80-b2018ca2ba52.png
│   │   ├── 8d5053aa-03fc-4f3d-ae80-b2018ca2ba52.webp
│   │   ├── 8d5053aa-03fc-4f3d-ae80-b2018ca2ba52_thumb.webp
│   │   ├── 8d547a99-ed58-4e85-93f5-bbb75bb83d4c.png
│   │   ├── 8d547a99-ed58-4e85-93f5-bbb75bb83d4c.webp
│   │   ├── 8d547a99-ed58-4e85-93f5-bbb75bb83d4c_thumb.webp
│   │   ├── 8ef22c7c-7f35-4bd1-8ba6-816e27efaee9.png
│   │   ├── 8ef22c7c-7f35-4bd1-8ba6-816e27efaee9.webp
│   │   ├── 8ef22c7c-7f35-4bd1-8ba6-816e27efaee9_thumb.webp
│   │   ├── 8f11aa97-6b7e-4220-b497-b7768e51bb01.png
│   │   ├── 8f11aa97-6b7e-4220-b497-b7768e51bb01.webp
│   │   ├── 8f11aa97-6b7e-4220-b497-b7768e51bb01_thumb.webp
│   │   ├── 8f3295f5-398e-4e58-858e-b4d0d3a9f149.png
│   │   ├── 8f3295f5-398e-4e58-858e-b4d0d3a9f149.webp
│   │   ├── 8f3295f5-398e-4e58-858e-b4d0d3a9f149_thumb.webp
│   │   ├── 8f4b7f92-f863-429e-8c91-b5e58b57a541.png
│   │   ├── 8f4b7f92-f863-429e-8c91-b5e58b57a541.webp
│   │   ├── 8f4b7f92-f863-429e-8c91-b5e58b57a541_thumb.webp
│   │   ├── 8fa5917a-01ee-4b43-9528-beab68828bca.png
│   │   ├── 8fa5917a-01ee-4b43-9528-beab68828bca.webp
│   │   ├── 8fa5917a-01ee-4b43-9528-beab68828bca_thumb.webp
│   │   ├── 906ce722-40e3-4dbf-8c04-add6e6ae924e.png
│   │   ├── 906ce722-40e3-4dbf-8c04-add6e6ae924e.webp
│   │   ├── 906ce722-40e3-4dbf-8c04-add6e6ae924e_thumb.webp
│   │   ├── 91d81602-8f50-49f1-bcc5-a6f04db99d99.png
│   │   ├── 91e4c4b3-9688-4326-9322-2287e3860612.png
│   │   ├── 91e4c4b3-9688-4326-9322-2287e3860612.webp
│   │   ├── 91e4c4b3-9688-4326-9322-2287e3860612_thumb.webp
│   │   ├── 92323e8c-83fb-4d48-a794-46993174974d.png
│   │   ├── 92323e8c-83fb-4d48-a794-46993174974d.webp
│   │   ├── 92323e8c-83fb-4d48-a794-46993174974d_thumb.webp
│   │   ├── 92446073-50d6-4b3e-b3cf-7365ec75d12a.png
│   │   ├── 92446073-50d6-4b3e-b3cf-7365ec75d12a.webp
│   │   ├── 92446073-50d6-4b3e-b3cf-7365ec75d12a_thumb.webp
│   │   ├── 93486f88-2d7b-42a6-9209-1e68ec708d82.png
│   │   ├── 93486f88-2d7b-42a6-9209-1e68ec708d82.webp
│   │   ├── 93486f88-2d7b-42a6-9209-1e68ec708d82_thumb.webp
│   │   ├── 935c1c9b-afba-4a08-9523-50deddf503f4.png
│   │   ├── 935c1c9b-afba-4a08-9523-50deddf503f4.webp
│   │   ├── 935c1c9b-afba-4a08-9523-50deddf503f4_thumb.webp
│   │   ├── 93fa35e9-e4d7-4eb2-84fd-a5f84820d62d.png
│   │   ├── 93fa35e9-e4d7-4eb2-84fd-a5f84820d62d.webp
│   │   ├── 93fa35e9-e4d7-4eb2-84fd-a5f84820d62d_thumb.webp
│   │   ├── 94e6782e-d5a0-490c-addb-1d0a7589bd65.png
│   │   ├── 94e6782e-d5a0-490c-addb-1d0a7589bd65.webp
│   │   ├── 94e6782e-d5a0-490c-addb-1d0a7589bd65_thumb.webp
│   │   ├── 95a7b00e-91b1-46c4-8575-8dc181e163d4.png
│   │   ├── 95a7b00e-91b1-46c4-8575-8dc181e163d4.webp
│   │   ├── 95a7b00e-91b1-46c4-8575-8dc181e163d4_thumb.webp
│   │   ├── 95aaf151-0823-489d-92ec-98fd93376513.png
│   │   ├── 95aaf151-0823-489d-92ec-98fd93376513.webp
│   │   ├── 95aaf151-0823-489d-92ec-98fd93376513_thumb.webp
│   │   ├── 96d744b2-835f-4eaf-b45e-5de7cc80a407.png
│   │   ├── 96d744b2-835f-4eaf-b45e-5de7cc80a407.webp
│   │   ├── 974440be-0f84-4c75-8337-74dd8c649bfe.png
│   │   ├── 974440be-0f84-4c75-8337-74dd8c649bfe.webp
│   │   ├── 974440be-0f84-4c75-8337-74dd8c649bfe_thumb.webp
│   │   ├── 97448ba4-beab-4414-be9d-8a9a58452046.png
│   │   ├── 97448ba4-beab-4414-be9d-8a9a58452046.webp
│   │   ├── 97448ba4-beab-4414-be9d-8a9a58452046_thumb.webp
│   │   ├── 97d28a6d-ec6b-433c-b478-311d6e1c1508.png
│   │   ├── 97d28a6d-ec6b-433c-b478-311d6e1c1508.webp
│   │   ├── 97d28a6d-ec6b-433c-b478-311d6e1c1508_thumb.webp
│   │   ├── 98064017-92c6-4394-9648-1f7ced4b1e4f.png
│   │   ├── 98064017-92c6-4394-9648-1f7ced4b1e4f.webp
│   │   ├── 98064017-92c6-4394-9648-1f7ced4b1e4f_thumb.webp
│   │   ├── 99333b0e-4849-4d49-80e7-4bd3587e7083.png
│   │   ├── 99333b0e-4849-4d49-80e7-4bd3587e7083.webp
│   │   ├── 99333b0e-4849-4d49-80e7-4bd3587e7083_thumb.webp
│   │   ├── 993c2fcb-a77f-4031-9c13-dc6fb20deeee.png
│   │   ├── 993c2fcb-a77f-4031-9c13-dc6fb20deeee.webp
│   │   ├── 99d566a9-b8a9-4c29-aca4-bfdad2511329.png
│   │   ├── 99d566a9-b8a9-4c29-aca4-bfdad2511329.webp
│   │   ├── 9a75dbb5-3b0a-4a93-bcb3-49bf150c0981.png
│   │   ├── 9a75dbb5-3b0a-4a93-bcb3-49bf150c0981.webp
│   │   ├── 9aba5a51-4c0a-4206-8c1f-4d966a104300.png
│   │   ├── 9aba5a51-4c0a-4206-8c1f-4d966a104300.webp
│   │   ├── 9aba5a51-4c0a-4206-8c1f-4d966a104300_thumb.webp
│   │   ├── 9af20ea7-4f09-4470-a822-e9c6ae1a7c5c.png
│   │   ├── 9af20ea7-4f09-4470-a822-e9c6ae1a7c5c.webp
│   │   ├── 9af20ea7-4f09-4470-a822-e9c6ae1a7c5c_thumb.webp
│   │   ├── 9bbca029-4b1a-4227-ba4e-8b6a44c0014d.png
│   │   ├── 9bbca029-4b1a-4227-ba4e-8b6a44c0014d.webp
│   │   ├── 9bbca029-4b1a-4227-ba4e-8b6a44c0014d_thumb.webp
│   │   ├── 9c99bf2a-74ee-4beb-b27d-16968392cf61.png
│   │   ├── 9c99bf2a-74ee-4beb-b27d-16968392cf61.webp
│   │   ├── 9c99bf2a-74ee-4beb-b27d-16968392cf61_thumb.webp
│   │   ├── 9d0a23e1-5bac-48d0-853e-37c0a6b6abb1.png
│   │   ├── 9d0a23e1-5bac-48d0-853e-37c0a6b6abb1.webp
│   │   ├── 9d0a23e1-5bac-48d0-853e-37c0a6b6abb1_thumb.webp
│   │   ├── 9d69a2f9-b196-48af-87df-d54767951f94.png
│   │   ├── 9d69a2f9-b196-48af-87df-d54767951f94.webp
│   │   ├── 9d69a2f9-b196-48af-87df-d54767951f94_thumb.webp
│   │   ├── 9dd5ce05-9a9c-4dbd-be21-b427540b9f81.png
│   │   ├── 9dd5ce05-9a9c-4dbd-be21-b427540b9f81.webp
│   │   ├── 9dd5ce05-9a9c-4dbd-be21-b427540b9f81_thumb.webp
│   │   ├── 9dd99459-1cc4-47f6-9b44-31e13656f6ca.png
│   │   ├── 9dd99459-1cc4-47f6-9b44-31e13656f6ca.webp
│   │   ├── 9dd99459-1cc4-47f6-9b44-31e13656f6ca_thumb.webp
│   │   ├── 9e014ec0-b782-452c-9268-5f53a0d7a08a.png
│   │   ├── 9e294f6d-52e7-4d52-8ef4-810d9a26d630.png
│   │   ├── 9e294f6d-52e7-4d52-8ef4-810d9a26d630.webp
│   │   ├── 9e294f6d-52e7-4d52-8ef4-810d9a26d630_thumb.webp
│   │   ├── 9e3cfdf6-57f0-41f1-859e-e2db3a871bce.png
│   │   ├── 9e3cfdf6-57f0-41f1-859e-e2db3a871bce.webp
│   │   ├── 9e3cfdf6-57f0-41f1-859e-e2db3a871bce_thumb.webp
│   │   ├── 9ef4685f-00a1-4ca4-8b1b-c6870523b0e2.png
│   │   ├── 9ef4685f-00a1-4ca4-8b1b-c6870523b0e2.webp
│   │   ├── 9ef4685f-00a1-4ca4-8b1b-c6870523b0e2_thumb.webp
│   │   ├── 9f2b7ead-ddf3-4943-867a-e465278ecb86.png
│   │   ├── 9f2b7ead-ddf3-4943-867a-e465278ecb86.webp
│   │   ├── 9fc0af30-dd0b-49de-a772-7350b8a14745.png
│   │   ├── 9fc0af30-dd0b-49de-a772-7350b8a14745.webp
│   │   ├── 9fc0af30-dd0b-49de-a772-7350b8a14745_thumb.webp
│   │   ├── a1bcc084-10a2-456c-89b9-5b3a29a04f29.png
│   │   ├── a1bcc084-10a2-456c-89b9-5b3a29a04f29.webp
│   │   ├── a1bcc084-10a2-456c-89b9-5b3a29a04f29_thumb.webp
│   │   ├── a235e6e0-074d-4544-90cc-958b1f48dcef.png
│   │   ├── a235e6e0-074d-4544-90cc-958b1f48dcef.webp
│   │   ├── a235e6e0-074d-4544-90cc-958b1f48dcef_thumb.webp
│   │   ├── a33ab519-d356-4800-80a5-634831e5bc01.png
│   │   ├── a33ab519-d356-4800-80a5-634831e5bc01.webp
│   │   ├── a33ab519-d356-4800-80a5-634831e5bc01_thumb.webp
│   │   ├── a390c666-f5b3-4479-9896-62930306ebf9.png
│   │   ├── a390c666-f5b3-4479-9896-62930306ebf9.txt
│   │   ├── a46a10e0-e138-415e-882e-d7755d880eb9.png
│   │   ├── a46a10e0-e138-415e-882e-d7755d880eb9.webp
│   │   ├── a46a10e0-e138-415e-882e-d7755d880eb9_thumb.webp
│   │   ├── a5423c81-eba6-4e19-be0d-65778e25dc99.png
│   │   ├── a5423c81-eba6-4e19-be0d-65778e25dc99.webp
│   │   ├── a5423c81-eba6-4e19-be0d-65778e25dc99_thumb.webp
│   │   ├── a5716bb9-9f58-4b09-8e2a-3c836087c6c4.png
│   │   ├── a5716bb9-9f58-4b09-8e2a-3c836087c6c4.webp
│   │   ├── a5716bb9-9f58-4b09-8e2a-3c836087c6c4_thumb.webp
│   │   ├── a6f4b534-17f3-4a63-b499-aaecdf2cac78.png
│   │   ├── a6f4b534-17f3-4a63-b499-aaecdf2cac78.webp
│   │   ├── a6f4b534-17f3-4a63-b499-aaecdf2cac78_thumb.webp
│   │   ├── a74ec392-1c31-412e-9447-389896b82ac5.png
│   │   ├── a74ec392-1c31-412e-9447-389896b82ac5.webp
│   │   ├── a74ec392-1c31-412e-9447-389896b82ac5_thumb.webp
│   │   ├── a7bab964-5f2c-4f05-8b52-d5067700e00a.png
│   │   ├── a7bab964-5f2c-4f05-8b52-d5067700e00a.webp
│   │   ├── a7bab964-5f2c-4f05-8b52-d5067700e00a_thumb.webp
│   │   ├── a8ad252d-08b0-489f-94e6-e4b5dbf7b1c9.png
│   │   ├── a8ad252d-08b0-489f-94e6-e4b5dbf7b1c9.webp
│   │   ├── a8ad252d-08b0-489f-94e6-e4b5dbf7b1c9_thumb.webp
│   │   ├── a9fa87df-b7a6-4b35-8176-77d64bba7e08.png
│   │   ├── a9fa87df-b7a6-4b35-8176-77d64bba7e08.webp
│   │   ├── a9fa87df-b7a6-4b35-8176-77d64bba7e08_thumb.webp
│   │   ├── aa56da3a-2f4d-43d2-92fc-98f6f683e1c5.png
│   │   ├── aa56da3a-2f4d-43d2-92fc-98f6f683e1c5.webp
│   │   ├── aa56da3a-2f4d-43d2-92fc-98f6f683e1c5_thumb.webp
│   │   ├── aa7b56c4-e28a-449f-b2be-dbe4ee9862dd.png
│   │   ├── aa7b56c4-e28a-449f-b2be-dbe4ee9862dd.webp
│   │   ├── aa7b56c4-e28a-449f-b2be-dbe4ee9862dd_thumb.webp
│   │   ├── aa9a385b-4532-4244-b5b6-84cff09087ac.png
│   │   ├── aa9a385b-4532-4244-b5b6-84cff09087ac.webp
│   │   ├── aa9a385b-4532-4244-b5b6-84cff09087ac_thumb.webp
│   │   ├── ab96291a-8ab0-416e-85fe-bf4582dff198.png
│   │   ├── ab96291a-8ab0-416e-85fe-bf4582dff198.webp
│   │   ├── ab96291a-8ab0-416e-85fe-bf4582dff198_thumb.webp
│   │   ├── abacbfbe-fd42-4083-acb1-1c04a51ec89b.png
│   │   ├── abacbfbe-fd42-4083-acb1-1c04a51ec89b.webp
│   │   ├── abacbfbe-fd42-4083-acb1-1c04a51ec89b_thumb.webp
│   │   ├── ac07fc25-67e5-4767-a479-60271ad1e067.png
│   │   ├── ac07fc25-67e5-4767-a479-60271ad1e067.webp
│   │   ├── ac07fc25-67e5-4767-a479-60271ad1e067_thumb.webp
│   │   ├── ac8d34c1-2b83-4e0c-9ccb-467a6913c5c3.png
│   │   ├── ac8d34c1-2b83-4e0c-9ccb-467a6913c5c3.webp
│   │   ├── ac8d34c1-2b83-4e0c-9ccb-467a6913c5c3_thumb.webp
│   │   ├── acd31f92-5d71-41c7-996c-84f455dbdc81.png
│   │   ├── acd31f92-5d71-41c7-996c-84f455dbdc81.webp
│   │   ├── acd31f92-5d71-41c7-996c-84f455dbdc81_thumb.webp
│   │   ├── acf3c8b4-20cb-4d1e-8b52-3222c462e57e.png
│   │   ├── acf3c8b4-20cb-4d1e-8b52-3222c462e57e.webp
│   │   ├── acf3c8b4-20cb-4d1e-8b52-3222c462e57e_thumb.webp
│   │   ├── ad270aad-e2ba-4bbb-842d-00e48d34deee.png
│   │   ├── ad270aad-e2ba-4bbb-842d-00e48d34deee.webp
│   │   ├── ad270aad-e2ba-4bbb-842d-00e48d34deee_thumb.webp
│   │   ├── ad8b953b-3aff-46e8-91d5-e4ccf4900897.png
│   │   ├── ad8b953b-3aff-46e8-91d5-e4ccf4900897.txt
│   │   ├── ad9101a0-c982-44fd-811d-54b4dd9b1f41.png
│   │   ├── ad9101a0-c982-44fd-811d-54b4dd9b1f41.webp
│   │   ├── aefabde3-d110-45ce-ac1c-8d6df1ec7be6.png
│   │   ├── aefabde3-d110-45ce-ac1c-8d6df1ec7be6.webp
│   │   ├── aefabde3-d110-45ce-ac1c-8d6df1ec7be6_thumb.webp
│   │   ├── af0658af-0eb8-4fc1-8215-12b47345a176.png
│   │   ├── af0658af-0eb8-4fc1-8215-12b47345a176.webp
│   │   ├── af0658af-0eb8-4fc1-8215-12b47345a176_thumb.webp
│   │   ├── ankycoin-0fe7ec19-b3d4-4e0f-bbff-aaf510c76848.png
│   │   ├── ankycoin-10b0b665-c3d1-4dd6-8606-e0124413604b.json
│   │   ├── ankycoin-10b0b665-c3d1-4dd6-8606-e0124413604b.png
│   │   ├── ankycoin-1f6a1237-13f6-4c29-9a0b-0167f405bc73.json
│   │   ├── ankycoin-1f6a1237-13f6-4c29-9a0b-0167f405bc73.png
│   │   ├── ankycoin-4c57ede6-5a66-4e02-82c2-00b09e8b6556.png
│   │   ├── ankycoin-7010f3ba-4ce2-4159-8d10-9e114959659d.json
│   │   ├── ankycoin-7010f3ba-4ce2-4159-8d10-9e114959659d.png
│   │   ├── b0ce1111-7175-4423-a8b4-1140bc84d9a5.png
│   │   ├── b0ce1111-7175-4423-a8b4-1140bc84d9a5.webp
│   │   ├── b3e38269-848c-46b8-ac86-8911c364a42e.png
│   │   ├── b3e38269-848c-46b8-ac86-8911c364a42e.webp
│   │   ├── b3e38269-848c-46b8-ac86-8911c364a42e_thumb.webp
│   │   ├── b535cfc0-dd5a-4e3e-8fab-899d0ed1cf93.png
│   │   ├── b535cfc0-dd5a-4e3e-8fab-899d0ed1cf93.webp
│   │   ├── b67efe35-59a4-4848-85c6-b6eb92c75718.png
│   │   ├── b67efe35-59a4-4848-85c6-b6eb92c75718.webp
│   │   ├── b67efe35-59a4-4848-85c6-b6eb92c75718_thumb.webp
│   │   ├── b69aa70e-eb1e-4129-a9c2-576e3a9bf1d2.png
│   │   ├── b69aa70e-eb1e-4129-a9c2-576e3a9bf1d2.webp
│   │   ├── b69aa70e-eb1e-4129-a9c2-576e3a9bf1d2_thumb.webp
│   │   ├── b76a8778-3d33-4510-9bfa-3fc1c5875e2e.png
│   │   ├── b76a8778-3d33-4510-9bfa-3fc1c5875e2e.webp
│   │   ├── b76a8778-3d33-4510-9bfa-3fc1c5875e2e_thumb.webp
│   │   ├── b8478f64-8461-45f6-9f4c-bb22f47d5b1d.png
│   │   ├── b8478f64-8461-45f6-9f4c-bb22f47d5b1d.webp
│   │   ├── b8478f64-8461-45f6-9f4c-bb22f47d5b1d_thumb.webp
│   │   ├── b8c7dc6d-153f-4c99-945b-025b9de841e1.png
│   │   ├── b8c7dc6d-153f-4c99-945b-025b9de841e1.webp
│   │   ├── b8c7dc6d-153f-4c99-945b-025b9de841e1_thumb.webp
│   │   ├── b9196f76-ac36-43ab-8a3e-2bb2be45f6cf.png
│   │   ├── b9196f76-ac36-43ab-8a3e-2bb2be45f6cf.webp
│   │   ├── b9196f76-ac36-43ab-8a3e-2bb2be45f6cf_thumb.webp
│   │   ├── b987cc3e-9e6a-491e-bbd5-8eba99c4d41a.png
│   │   ├── b987cc3e-9e6a-491e-bbd5-8eba99c4d41a.txt
│   │   ├── bc03c8a6-38e7-4cb7-922d-72c369b66bd1.png
│   │   ├── bc03c8a6-38e7-4cb7-922d-72c369b66bd1.webp
│   │   ├── bc03c8a6-38e7-4cb7-922d-72c369b66bd1_thumb.webp
│   │   ├── bc19ccf5-eaa1-4b89-860a-dd8986f897e5.png
│   │   ├── be060200-aa19-4dcc-bbee-05a12c0cc730.png
│   │   ├── be060200-aa19-4dcc-bbee-05a12c0cc730.webp
│   │   ├── be060200-aa19-4dcc-bbee-05a12c0cc730_thumb.webp
│   │   ├── be393f81-df4d-4e1b-a33e-e4ace39e0345.png
│   │   ├── be393f81-df4d-4e1b-a33e-e4ace39e0345.webp
│   │   ├── be393f81-df4d-4e1b-a33e-e4ace39e0345_thumb.webp
│   │   ├── bee3a191-29a1-4f3f-8b6c-3a44de57aa4e.png
│   │   ├── bf2fd41a-eb5b-4867-8ba7-5e63f1102bb4.png
│   │   ├── bf2fd41a-eb5b-4867-8ba7-5e63f1102bb4.webp
│   │   ├── bf2fd41a-eb5b-4867-8ba7-5e63f1102bb4_thumb.webp
│   │   ├── bf6fad53-2af6-4f6c-ad70-75a9ec67d389.png
│   │   ├── bf6fad53-2af6-4f6c-ad70-75a9ec67d389.webp
│   │   ├── bf6fad53-2af6-4f6c-ad70-75a9ec67d389_thumb.webp
│   │   ├── c0c56d5f-7755-4d00-9620-0567fac47c19.png
│   │   ├── c0c56d5f-7755-4d00-9620-0567fac47c19.webp
│   │   ├── c0c56d5f-7755-4d00-9620-0567fac47c19_thumb.webp
│   │   ├── c16693cd-9d46-44d8-bf89-c3fb7e4d1376.png
│   │   ├── c16693cd-9d46-44d8-bf89-c3fb7e4d1376.webp
│   │   ├── c16693cd-9d46-44d8-bf89-c3fb7e4d1376_thumb.webp
│   │   ├── c1c9e3fc-17d0-4bac-9ca8-68475b2dafa6.png
│   │   ├── c1c9e3fc-17d0-4bac-9ca8-68475b2dafa6.webp
│   │   ├── c1c9e3fc-17d0-4bac-9ca8-68475b2dafa6_thumb.webp
│   │   ├── c1d73dbf-67a2-4b6f-a43c-9a9e250148bc.png
│   │   ├── c1d73dbf-67a2-4b6f-a43c-9a9e250148bc.webp
│   │   ├── c1d73dbf-67a2-4b6f-a43c-9a9e250148bc_thumb.webp
│   │   ├── c30b1c27-bef3-45f1-b3ad-2c8fc5fa0733.png
│   │   ├── c30b1c27-bef3-45f1-b3ad-2c8fc5fa0733.webp
│   │   ├── c30b1c27-bef3-45f1-b3ad-2c8fc5fa0733_thumb.webp
│   │   ├── c35daf9a-6ab5-46be-938a-25618987b709.png
│   │   ├── c5f36ce9-1255-4c27-ac4a-f99c9e4b8796.png
│   │   ├── c5f36ce9-1255-4c27-ac4a-f99c9e4b8796.webp
│   │   ├── c5f36ce9-1255-4c27-ac4a-f99c9e4b8796_thumb.webp
│   │   ├── c5fdaf4b-50c1-4f74-9a00-f347ec03d353.png
│   │   ├── c60c500f-ccfd-4bbf-ab9d-81c52d48b2ee.png
│   │   ├── c60c500f-ccfd-4bbf-ab9d-81c52d48b2ee.webp
│   │   ├── c60c500f-ccfd-4bbf-ab9d-81c52d48b2ee_thumb.webp
│   │   ├── c69ca009-4abf-4bab-84f5-ce1977df8042.png
│   │   ├── c69ca009-4abf-4bab-84f5-ce1977df8042.webp
│   │   ├── c69ca009-4abf-4bab-84f5-ce1977df8042_thumb.webp
│   │   ├── c92adf51-ca43-416f-9f0f-db48f7523026.png
│   │   ├── c92adf51-ca43-416f-9f0f-db48f7523026.webp
│   │   ├── c92adf51-ca43-416f-9f0f-db48f7523026_thumb.webp
│   │   ├── c963c8d9-dc52-4fe7-b1fc-c50864a484f4.png
│   │   ├── c963c8d9-dc52-4fe7-b1fc-c50864a484f4.webp
│   │   ├── c963c8d9-dc52-4fe7-b1fc-c50864a484f4_thumb.webp
│   │   ├── ca4e92b0-8d98-45e9-825f-ed857b28b862.png
│   │   ├── ca4e92b0-8d98-45e9-825f-ed857b28b862.webp
│   │   ├── ca4e92b0-8d98-45e9-825f-ed857b28b862_thumb.webp
│   │   ├── cb881adc-d92d-47d4-8688-05d34ee052fa.png
│   │   ├── cb881adc-d92d-47d4-8688-05d34ee052fa.webp
│   │   ├── cb881adc-d92d-47d4-8688-05d34ee052fa_thumb.webp
│   │   ├── cbd512da-65ab-4ed6-9020-f94e7869f242.png
│   │   ├── cbd512da-65ab-4ed6-9020-f94e7869f242.webp
│   │   ├── cbd512da-65ab-4ed6-9020-f94e7869f242_thumb.webp
│   │   ├── cc0c887e-e8fd-417f-9768-b0e4e42110ab.png
│   │   ├── cc0c887e-e8fd-417f-9768-b0e4e42110ab.webp
│   │   ├── cc0c887e-e8fd-417f-9768-b0e4e42110ab_thumb.webp
│   │   ├── cc91b3e1-1e5d-40ed-bd2f-7b55c65f878c.png
│   │   ├── cc91b3e1-1e5d-40ed-bd2f-7b55c65f878c.webp
│   │   ├── cc91b3e1-1e5d-40ed-bd2f-7b55c65f878c_thumb.webp
│   │   ├── cd0a098a-2684-44bc-9343-486297d46b2e.png
│   │   ├── cd0a098a-2684-44bc-9343-486297d46b2e.webp
│   │   ├── cd807a4e-b73d-4196-963d-950997bd03b6.png
│   │   ├── cd807a4e-b73d-4196-963d-950997bd03b6.webp
│   │   ├── cd807a4e-b73d-4196-963d-950997bd03b6_thumb.webp
│   │   ├── cdaa2198-adcc-4dd5-b941-416998aa21ad.png
│   │   ├── cdaa2198-adcc-4dd5-b941-416998aa21ad.webp
│   │   ├── cdaa2198-adcc-4dd5-b941-416998aa21ad_thumb.webp
│   │   ├── cebebb21-be3e-4bfd-bde7-09e2bbb1e25a.png
│   │   ├── cebebb21-be3e-4bfd-bde7-09e2bbb1e25a.webp
│   │   ├── cebebb21-be3e-4bfd-bde7-09e2bbb1e25a_thumb.webp
│   │   ├── cedd3ffb-eb5a-4956-bb5d-1e10e4603f74.png
│   │   ├── cedd3ffb-eb5a-4956-bb5d-1e10e4603f74.webp
│   │   ├── cedd3ffb-eb5a-4956-bb5d-1e10e4603f74_thumb.webp
│   │   ├── ceed3936-0b60-4e90-88e3-f804e7e34e02.png
│   │   ├── ceed3936-0b60-4e90-88e3-f804e7e34e02.webp
│   │   ├── ceed3936-0b60-4e90-88e3-f804e7e34e02_thumb.webp
│   │   ├── cef94039-acc7-497d-8538-339abc6687fa.png
│   │   ├── cef94039-acc7-497d-8538-339abc6687fa.webp
│   │   ├── cef94039-acc7-497d-8538-339abc6687fa_thumb.webp
│   │   ├── create-video-bakery-opening-bell.jpg
│   │   ├── create-video-bakery-opening-bell.png
│   │   ├── create-video-bedroom-journal.jpg
│   │   ├── create-video-bedroom-journal.png
│   │   ├── create-video-bookstore-whisper.jpg
│   │   ├── create-video-bookstore-whisper.png
│   │   ├── create-video-boxing-gym-corner.jpg
│   │   ├── create-video-boxing-gym-corner.png
│   │   ├── create-video-bus-stop-rain.jpg
│   │   ├── create-video-bus-stop-rain.png
│   │   ├── create-video-classroom-after-hours.jpg
│   │   ├── create-video-classroom-after-hours.png
│   │   ├── create-video-community-garden.jpg
│   │   ├── create-video-community-garden.png
│   │   ├── create-video-dance-studio.jpg
│   │   ├── create-video-dance-studio.png
│   │   ├── create-video-diner-listen.jpg
│   │   ├── create-video-diner-listen.png
│   │   ├── create-video-empty-apartment-first-night.jpg
│   │   ├── create-video-empty-apartment-first-night.png
│   │   ├── create-video-family-dinner.jpg
│   │   ├── create-video-family-dinner.png
│   │   ├── create-video-fire-escape-voicemail.jpg
│   │   ├── create-video-fire-escape-voicemail.png
│   │   ├── create-video-grocery-aisle.jpg
│   │   ├── create-video-grocery-aisle.png
│   │   ├── create-video-haircut-mirror.jpg
│   │   ├── create-video-haircut-mirror.png
│   │   ├── create-video-hospital-corridor.jpg
│   │   ├── create-video-hospital-corridor.png
│   │   ├── create-video-kitchen-tea.jpg
│   │   ├── create-video-kitchen-tea.png
│   │   ├── create-video-laundromat-fold.jpg
│   │   ├── create-video-laundromat-fold.png
│   │   ├── create-video-mechanic-garage.jpg
│   │   ├── create-video-mechanic-garage.png
│   │   ├── create-video-office-stairwell.jpg
│   │   ├── create-video-office-stairwell.png
│   │   ├── create-video-park-bench-breakup.jpg
│   │   ├── create-video-park-bench-breakup.png
│   │   ├── create-video-recording-booth-first-true-take.jpg
│   │   ├── create-video-recording-booth-first-true-take.png
│   │   ├── create-video-rooftop-sunrise.jpg
│   │   ├── create-video-rooftop-sunrise.png
│   │   ├── create-video-seaside-walk.jpg
│   │   ├── create-video-seaside-walk.png
│   │   ├── create-video-shelter-adoption-moment.jpg
│   │   ├── create-video-shelter-adoption-moment.png
│   │   ├── create-video-sidewalk-chalk.jpg
│   │   ├── create-video-sidewalk-chalk.png
│   │   ├── create-video-subway-window.jpg
│   │   ├── create-video-subway-window.png
│   │   ├── create-video-thrift-store-new-self.jpg
│   │   ├── create-video-thrift-store-new-self.png
│   │   ├── create-video-wedding-speech-side-room.jpg
│   │   ├── create-video-wedding-speech-side-room.png
│   │   ├── d07514e9-7b20-4ebe-9ae4-154f0860f117.png
│   │   ├── d07514e9-7b20-4ebe-9ae4-154f0860f117.webp
│   │   ├── d07514e9-7b20-4ebe-9ae4-154f0860f117_thumb.webp
│   │   ├── d19a78b0-237f-4768-a794-4612b8d3e907.png
│   │   ├── d19a78b0-237f-4768-a794-4612b8d3e907.webp
│   │   ├── d2742b96-c621-4a3d-b725-b389cf74a1d5.png
│   │   ├── d2742b96-c621-4a3d-b725-b389cf74a1d5.webp
│   │   ├── d2742b96-c621-4a3d-b725-b389cf74a1d5_thumb.webp
│   │   ├── d3228151-195d-47da-bb6c-ef432675dbcf.png
│   │   ├── d3228151-195d-47da-bb6c-ef432675dbcf.webp
│   │   ├── d3228151-195d-47da-bb6c-ef432675dbcf_thumb.webp
│   │   ├── d3b0006f-0c6a-49af-983b-efd2673621b3.png
│   │   ├── d3b0006f-0c6a-49af-983b-efd2673621b3.webp
│   │   ├── d3b0006f-0c6a-49af-983b-efd2673621b3_thumb.webp
│   │   ├── d3d3629a-ae38-4292-a8a3-99adeda89cb5.png
│   │   ├── d3d3629a-ae38-4292-a8a3-99adeda89cb5.webp
│   │   ├── d42e3956-1c48-4d15-bc8f-c231eef27acd.png
│   │   ├── d42e3956-1c48-4d15-bc8f-c231eef27acd.webp
│   │   ├── d42e3956-1c48-4d15-bc8f-c231eef27acd_thumb.webp
│   │   ├── d44355bc-9db3-432d-86a3-f6b6aad65923.png
│   │   ├── d44355bc-9db3-432d-86a3-f6b6aad65923.webp
│   │   ├── d44355bc-9db3-432d-86a3-f6b6aad65923_thumb.webp
│   │   ├── d4ac7a20-9dc2-4af0-93d0-f2470fb92a39.png
│   │   ├── d4ac7a20-9dc2-4af0-93d0-f2470fb92a39.webp
│   │   ├── d4ac7a20-9dc2-4af0-93d0-f2470fb92a39_thumb.webp
│   │   ├── d5525129-55d7-4815-8e0a-7f911c736690.png
│   │   ├── d5525129-55d7-4815-8e0a-7f911c736690.webp
│   │   ├── d5525129-55d7-4815-8e0a-7f911c736690_thumb.webp
│   │   ├── d56243df-8dc2-4f2c-9efc-10a2d03abfdb.png
│   │   ├── d56243df-8dc2-4f2c-9efc-10a2d03abfdb.webp
│   │   ├── d56243df-8dc2-4f2c-9efc-10a2d03abfdb_thumb.webp
│   │   ├── d5a150c9-c3bf-475c-940e-39a38e282843.png
│   │   ├── d5a150c9-c3bf-475c-940e-39a38e282843.webp
│   │   ├── d5a150c9-c3bf-475c-940e-39a38e282843_thumb.webp
│   │   ├── d6174d74-0be4-431a-a54d-a98eb241d08f.png
│   │   ├── d6174d74-0be4-431a-a54d-a98eb241d08f.webp
│   │   ├── d6174d74-0be4-431a-a54d-a98eb241d08f_thumb.webp
│   │   ├── d6eca22f-b2e2-4c8d-9de7-ace5795e8576.png
│   │   ├── d6eca22f-b2e2-4c8d-9de7-ace5795e8576.webp
│   │   ├── d6eca22f-b2e2-4c8d-9de7-ace5795e8576_thumb.webp
│   │   ├── d6eda782-5fe3-48e6-98be-8d3be42e3e85.png
│   │   ├── d6eda782-5fe3-48e6-98be-8d3be42e3e85.webp
│   │   ├── d6eda782-5fe3-48e6-98be-8d3be42e3e85_thumb.webp
│   │   ├── d7b48af5-f64a-46ee-9195-d01e38faa548.png
│   │   ├── d7b48af5-f64a-46ee-9195-d01e38faa548.webp
│   │   ├── d7b48af5-f64a-46ee-9195-d01e38faa548_thumb.webp
│   │   ├── d8444e7a-3ac9-4752-9b44-c1ee0150943a.png
│   │   ├── da4b7716-afca-4a56-aab7-03c535a5ffca.png
│   │   ├── da4b7716-afca-4a56-aab7-03c535a5ffca.webp
│   │   ├── da4b7716-afca-4a56-aab7-03c535a5ffca_thumb.webp
│   │   ├── dc02b7aa-fa18-4ed7-8f56-ef2c698096fa.png
│   │   ├── dc02b7aa-fa18-4ed7-8f56-ef2c698096fa.webp
│   │   ├── dc02b7aa-fa18-4ed7-8f56-ef2c698096fa_thumb.webp
│   │   ├── dca157c3-03a2-4ef0-8cb6-379b32ad0ffb.png
│   │   ├── dca157c3-03a2-4ef0-8cb6-379b32ad0ffb.webp
│   │   ├── dca157c3-03a2-4ef0-8cb6-379b32ad0ffb_thumb.webp
│   │   ├── dccc3619-9c8a-4250-8523-9b9642538a12.png
│   │   ├── dccc3619-9c8a-4250-8523-9b9642538a12.webp
│   │   ├── dccc3619-9c8a-4250-8523-9b9642538a12_thumb.webp
│   │   ├── df74ea33-1f31-4ba4-b17a-f60591e67230.png
│   │   ├── df74ea33-1f31-4ba4-b17a-f60591e67230.webp
│   │   ├── df74ea33-1f31-4ba4-b17a-f60591e67230_thumb.webp
│   │   ├── e0d3c8f6-7250-4b35-bdcd-763c161df666.png
│   │   ├── e0fe1585-e179-44b6-9106-9921dbe7c581.png
│   │   ├── e0fe1585-e179-44b6-9106-9921dbe7c581.webp
│   │   ├── e0fe1585-e179-44b6-9106-9921dbe7c581_thumb.webp
│   │   ├── e11998bd-b8ee-42c0-8078-77a8ec733a1d.png
│   │   ├── e11998bd-b8ee-42c0-8078-77a8ec733a1d.webp
│   │   ├── e11998bd-b8ee-42c0-8078-77a8ec733a1d_thumb.webp
│   │   ├── e17814af-9779-4357-b723-7dc4f3dd7716.png
│   │   ├── e17814af-9779-4357-b723-7dc4f3dd7716.webp
│   │   ├── e17814af-9779-4357-b723-7dc4f3dd7716_thumb.webp
│   │   ├── e317819a-4765-4475-8c30-83f579e0f6e1.png
│   │   ├── e317819a-4765-4475-8c30-83f579e0f6e1.webp
│   │   ├── e317819a-4765-4475-8c30-83f579e0f6e1_thumb.webp
│   │   ├── e425dbc5-6873-40af-b46f-dc5d13cbfbc0.png
│   │   ├── e425dbc5-6873-40af-b46f-dc5d13cbfbc0.webp
│   │   ├── e425dbc5-6873-40af-b46f-dc5d13cbfbc0_thumb.webp
│   │   ├── e4402c5f-6c46-4f8c-bca3-243beeaef6d1.png
│   │   ├── e4402c5f-6c46-4f8c-bca3-243beeaef6d1.webp
│   │   ├── e4402c5f-6c46-4f8c-bca3-243beeaef6d1_thumb.webp
│   │   ├── e51e0d1d-f840-4a67-9bfc-b388a07903de.png
│   │   ├── e5527311-1668-4aea-b9ee-2ccaa76500bb.png
│   │   ├── e5527311-1668-4aea-b9ee-2ccaa76500bb.webp
│   │   ├── e5527311-1668-4aea-b9ee-2ccaa76500bb_thumb.webp
│   │   ├── e55660c3-8cd7-440f-ae26-0320218f6262.png
│   │   ├── e55660c3-8cd7-440f-ae26-0320218f6262.webp
│   │   ├── e55660c3-8cd7-440f-ae26-0320218f6262_thumb.webp
│   │   ├── e605fac1-21d2-4313-bce8-1e5c8c9cb6fc.png
│   │   ├── e605fac1-21d2-4313-bce8-1e5c8c9cb6fc.webp
│   │   ├── e605fac1-21d2-4313-bce8-1e5c8c9cb6fc_thumb.webp
│   │   ├── e63d56ac-38b4-416d-830f-fec1efd2ae15.png
│   │   ├── e63d56ac-38b4-416d-830f-fec1efd2ae15.webp
│   │   ├── e63d56ac-38b4-416d-830f-fec1efd2ae15_thumb.webp
│   │   ├── e6b85adc-09e5-4e76-a056-2a15d7142b95.png
│   │   ├── e6b85adc-09e5-4e76-a056-2a15d7142b95.webp
│   │   ├── e6b85adc-09e5-4e76-a056-2a15d7142b95_thumb.webp
│   │   ├── e738de0f-11fc-4f71-ae73-d5d8f6f68da8.png
│   │   ├── e738de0f-11fc-4f71-ae73-d5d8f6f68da8.webp
│   │   ├── e738de0f-11fc-4f71-ae73-d5d8f6f68da8_thumb.webp
│   │   ├── e8239af7-5cfe-4dfb-af74-f7b55a3de65e.png
│   │   ├── e8239af7-5cfe-4dfb-af74-f7b55a3de65e.webp
│   │   ├── e8239af7-5cfe-4dfb-af74-f7b55a3de65e_thumb.webp
│   │   ├── e8c7aca1-496c-4102-82e3-c5b58b67119f.png
│   │   ├── e8c7aca1-496c-4102-82e3-c5b58b67119f.webp
│   │   ├── e8c7aca1-496c-4102-82e3-c5b58b67119f_thumb.webp
│   │   ├── e943624b-d44a-4b09-b8f4-346805aff181.png
│   │   ├── e943624b-d44a-4b09-b8f4-346805aff181.webp
│   │   ├── e943624b-d44a-4b09-b8f4-346805aff181_thumb.webp
│   │   ├── e9db46ee-16e6-44de-ae56-0faed7d54154.png
│   │   ├── e9db46ee-16e6-44de-ae56-0faed7d54154.webp
│   │   ├── e9db46ee-16e6-44de-ae56-0faed7d54154_thumb.webp
│   │   ├── ea943b23-b42c-4f02-925a-2d847e8e840f.png
│   │   ├── ea943b23-b42c-4f02-925a-2d847e8e840f.webp
│   │   ├── eb411765-4c61-4573-ab74-b821b18e4bc0.png
│   │   ├── eb411765-4c61-4573-ab74-b821b18e4bc0.webp
│   │   ├── eb411765-4c61-4573-ab74-b821b18e4bc0_thumb.webp
│   │   ├── ebce7458-d1eb-44a9-969c-0fd88b784afb.png
│   │   ├── ebce7458-d1eb-44a9-969c-0fd88b784afb.webp
│   │   ├── ec4eae86-2dd6-4279-9cec-e9fd9f67f214.png
│   │   ├── ec4eae86-2dd6-4279-9cec-e9fd9f67f214.webp
│   │   ├── ec4eae86-2dd6-4279-9cec-e9fd9f67f214_thumb.webp
│   │   ├── ecbf45e4-dcf5-47ee-a1dd-4758ca2b8ed5.png
│   │   ├── eda98675-bfe9-42ad-8e61-503534945c86.png
│   │   ├── eda98675-bfe9-42ad-8e61-503534945c86.webp
│   │   ├── edf7676c-ac5d-497a-835e-64ae5123b67e.png
│   │   ├── edf7676c-ac5d-497a-835e-64ae5123b67e.webp
│   │   ├── edf7676c-ac5d-497a-835e-64ae5123b67e_thumb.webp
│   │   ├── ee6e74ab-4815-4c3c-b280-76b14aa2e060.png
│   │   ├── ee6e74ab-4815-4c3c-b280-76b14aa2e060.webp
│   │   ├── ee6e74ab-4815-4c3c-b280-76b14aa2e060_thumb.webp
│   │   ├── ef481b14-9381-4c9e-a0f0-a27b8ffd1b96.png
│   │   ├── ef481b14-9381-4c9e-a0f0-a27b8ffd1b96.webp
│   │   ├── ef481b14-9381-4c9e-a0f0-a27b8ffd1b96_thumb.webp
│   │   ├── ef6e7b48-df11-4207-be13-eba21aa718ce.png
│   │   ├── ef6e7b48-df11-4207-be13-eba21aa718ce.webp
│   │   ├── ef6e7b48-df11-4207-be13-eba21aa718ce_thumb.webp
│   │   ├── f04ab63e-89c2-4c36-a5b5-fa371c7c6cab.png
│   │   ├── f04ab63e-89c2-4c36-a5b5-fa371c7c6cab.webp
│   │   ├── f04ab63e-89c2-4c36-a5b5-fa371c7c6cab_thumb.webp
│   │   ├── f101962a-aa12-471f-9021-a23333e54642.png
│   │   ├── f101962a-aa12-471f-9021-a23333e54642.webp
│   │   ├── f101962a-aa12-471f-9021-a23333e54642_thumb.webp
│   │   ├── f17d0073-d108-4840-b833-b37f47c221b5.png
│   │   ├── f17d0073-d108-4840-b833-b37f47c221b5.webp
│   │   ├── f17d0073-d108-4840-b833-b37f47c221b5_thumb.webp
│   │   ├── f1ef3c43-2614-48e8-9b1e-6c4332d47d5c.png
│   │   ├── f215265f-c9f0-4884-9b38-ebb4fa39dda0.png
│   │   ├── f215265f-c9f0-4884-9b38-ebb4fa39dda0.webp
│   │   ├── f22b53e0-7fa0-444f-86c7-0426f42e1da3.png
│   │   ├── f22b53e0-7fa0-444f-86c7-0426f42e1da3.webp
│   │   ├── f22b53e0-7fa0-444f-86c7-0426f42e1da3_thumb.webp
│   │   ├── f3ebab9c-de3e-45dc-9bae-9888ae4ef8fb.png
│   │   ├── f3ebab9c-de3e-45dc-9bae-9888ae4ef8fb.webp
│   │   ├── f3ebab9c-de3e-45dc-9bae-9888ae4ef8fb_thumb.webp
│   │   ├── f48d194e-1777-40b0-8a74-cb6770783bc3.png
│   │   ├── f48d194e-1777-40b0-8a74-cb6770783bc3.webp
│   │   ├── f48d194e-1777-40b0-8a74-cb6770783bc3_thumb.webp
│   │   ├── f6bece74-272c-4b2b-b77a-8577ee2dd98e.png
│   │   ├── f6bece74-272c-4b2b-b77a-8577ee2dd98e.webp
│   │   ├── f6bece74-272c-4b2b-b77a-8577ee2dd98e_thumb.webp
│   │   ├── f7199d5f-f298-4edb-ba4b-f2714ed3738b.png
│   │   ├── f7199d5f-f298-4edb-ba4b-f2714ed3738b.webp
│   │   ├── f7199d5f-f298-4edb-ba4b-f2714ed3738b_thumb.webp
│   │   ├── f7b25c05-8b87-4734-b69e-6eade15113b0.png
│   │   ├── f7b25c05-8b87-4734-b69e-6eade15113b0.webp
│   │   ├── f7b25c05-8b87-4734-b69e-6eade15113b0_thumb.webp
│   │   ├── f7f99255-356c-492f-a50a-c740e7aaf540.png
│   │   ├── f7f99255-356c-492f-a50a-c740e7aaf540.webp
│   │   ├── f7f99255-356c-492f-a50a-c740e7aaf540_thumb.webp
│   │   ├── f870564a-cfe9-4d6a-83c0-73dec7f34e4f.png
│   │   ├── f870564a-cfe9-4d6a-83c0-73dec7f34e4f.txt
│   │   ├── fa21b2e0-6a26-4e31-a654-13f70b65a17f.png
│   │   ├── fa345500-4bcd-47cf-822d-4f4feaaf5656.png
│   │   ├── fa345500-4bcd-47cf-822d-4f4feaaf5656.webp
│   │   ├── fa345500-4bcd-47cf-822d-4f4feaaf5656_thumb.webp
│   │   ├── farcaster
│   │   ├── fb50e90a-981c-4cbd-9fb3-be6a29f21896.png
│   │   ├── fb50e90a-981c-4cbd-9fb3-be6a29f21896.webp
│   │   ├── fb50e90a-981c-4cbd-9fb3-be6a29f21896_thumb.webp
│   │   ├── fba2d4fe-7aba-44c6-ba82-fa5a4351fe68.png
│   │   ├── fba2d4fe-7aba-44c6-ba82-fa5a4351fe68.webp
│   │   ├── fba2d4fe-7aba-44c6-ba82-fa5a4351fe68_thumb.webp
│   │   ├── fc360087-6b83-4116-b2c9-267cb7830178.png
│   │   ├── fc360087-6b83-4116-b2c9-267cb7830178.webp
│   │   ├── fc360087-6b83-4116-b2c9-267cb7830178_thumb.webp
│   │   ├── fc85f235-8378-423b-ae19-b0140396c969.png
│   │   ├── fc85f235-8378-423b-ae19-b0140396c969.webp
│   │   ├── fd6bfb20-60f5-48f3-823a-56bba87d9357.png
│   │   ├── fd6bfb20-60f5-48f3-823a-56bba87d9357.webp
│   │   ├── fd6bfb20-60f5-48f3-823a-56bba87d9357_thumb.webp
│   │   ├── fdec320d-0d5d-4b6f-8fc9-7120a9d6ef45.png
│   │   ├── fdec320d-0d5d-4b6f-8fc9-7120a9d6ef45.webp
│   │   ├── fdec320d-0d5d-4b6f-8fc9-7120a9d6ef45_thumb.webp
│   │   ├── fef3dadf-cdf2-460f-99bd-bd79b01874ec.png
│   │   ├── fef3dadf-cdf2-460f-99bd-bd79b01874ec.webp
│   │   ├── fef3dadf-cdf2-460f-99bd-bd79b01874ec_thumb.webp
│   │   ├── fefebad8-3ae5-4ba0-981a-949eca820456.png
│   │   ├── fefebad8-3ae5-4ba0-981a-949eca820456.webp
│   │   ├── fefebad8-3ae5-4ba0-981a-949eca820456_thumb.webp
│   │   ├── ff058a6c-79a6-4589-843f-aeecc47bfc3b.png
│   │   ├── ff058a6c-79a6-4589-843f-aeecc47bfc3b.webp
│   │   ├── landing_gifs
│   │   ├── mf-0484bab7-eb87-491f-bc06-f6761e063e46.png
│   │   ├── mf-1559bf69-4cb1-49b7-905d-81a9ac7cb03c.png
│   │   ├── mf-6cbfe654-9cee-423f-b1d9-ad761309c35f.png
│   │   ├── mf-8e4e1759-6584-4091-b321-06de3ca9b3e2.png
│   │   ├── mf-bfdfae4d-557e-48b4-aba3-c1684c0ab6aa.png
│   │   ├── mf-d4eb13b6-120e-4f1f-a278-528056d4c44b.png
│   │   ├── mf-e46705ce-57e4-4215-9d2f-8b361581feab.png
│   │   ├── mf-fd2dc0be-ea95-4510-a8e1-1381368d35a7.png
│   │   ├── mf-flux-391638ce-1d48-4412-afcd-7d174e1f8719.png
│   │   ├── mf-flux-586c2566-5304-4974-92f9-e092ad9c31c5.png
│   │   ├── mf-flux-fa1e1251-8c69-464e-a297-1200c9dbb04b.png
│   │   ├── pinned_20260312_114033.png
│   │   ├── prompt_1f9fc701-0856-47f5-bf6a-6033a1281aad.png
│   │   ├── prompt_27572679-9a22-42dd-b0de-3e73eb235d32.png
│   │   ├── prompt_430f0f57-65bd-4579-bfe7-926e698b2ccd.png
│   │   ├── prompt_5de42a6f-68d9-4f92-916b-4edc33afd84a.png
│   │   ├── prompt_5ed2d001-a0ae-4aaa-83ae-a47398e4754f.png
│   │   ├── prompt_9dcdaeae-2f86-4e0d-9078-785c6c0ed614.png
│   │   ├── prompt_fb067b52-dd7b-4762-a03d-68a76316d216.png
│   │   ├── prompt_fd84cd62-c15a-43cb-bd6c-0585d83bbab6.png
│   │   ├── rejected
│   │   ├── sample_cosmic-birth.png
│   │   ├── sample_cosmic-birth.txt
│   │   ├── sample_forest-meditation.png
│   │   ├── sample_forest-meditation.txt
│   │   ├── sample_ocean-dissolution.png
│   │   ├── sample_ocean-dissolution.txt
│   │   ├── sample_urban-neon-ghost.png
│   │   ├── sample_urban-neon-ghost.txt
│   │   ├── sample_writing-communion.png
│   │   ├── sample_writing-communion.txt
│   │   ├── thumbs
│   │   ├── video_0f341deb_00.png
│   │   ├── video_0f341deb_00.txt
│   │   ├── video_0f341deb_01.png
│   │   ├── video_0f341deb_01.txt
│   │   ├── video_0f341deb_02.png
│   │   ├── video_0f341deb_02.txt
│   │   ├── video_0f341deb_03.png
│   │   ├── video_0f341deb_03.txt
│   │   ├── video_0f341deb_04.png
│   │   ├── video_0f341deb_04.txt
│   │   ├── video_0f341deb_05.png
│   │   ├── video_0f341deb_05.txt
│   │   ├── video_0f341deb_06.png
│   │   ├── video_0f341deb_06.txt
│   │   ├── video_0f341deb_07.png
│   │   ├── video_0f341deb_07.txt
│   │   ├── video_0f341deb_08.png
│   │   ├── video_0f341deb_08.txt
│   │   ├── video_0f341deb_09.png
│   │   ├── video_0f341deb_09.txt
│   │   ├── video_0f341deb_10.png
│   │   ├── video_0f341deb_10.txt
│   │   ├── video_0f341deb_11.png
│   │   ├── video_0f341deb_11.txt
│   │   ├── video_1a5e4365_00.png
│   │   ├── video_1a5e4365_00.txt
│   │   ├── video_1a5e4365_01.png
│   │   ├── video_1a5e4365_01.txt
│   │   ├── video_1a5e4365_02.png
│   │   ├── video_1a5e4365_02.txt
│   │   ├── video_1a5e4365_03.png
│   │   ├── video_1a5e4365_03.txt
│   │   ├── video_1a5e4365_04.png
│   │   ├── video_1a5e4365_04.txt
│   │   ├── video_1a5e4365_05.png
│   │   ├── video_1a5e4365_05.txt
│   │   ├── video_1a5e4365_06.png
│   │   ├── video_1a5e4365_06.txt
│   │   ├── video_1a5e4365_07.png
│   │   ├── video_1a5e4365_07.txt
│   │   ├── video_1a5e4365_08.png
│   │   ├── video_1a5e4365_08.txt
│   │   ├── video_1a5e4365_09.png
│   │   ├── video_1a5e4365_09.txt
│   │   ├── video_1e65a5cb_00.png
│   │   ├── video_1e65a5cb_00.txt
│   │   ├── video_1e65a5cb_01.png
│   │   ├── video_1e65a5cb_01.txt
│   │   ├── video_1e65a5cb_02.png
│   │   ├── video_1e65a5cb_02.txt
│   │   ├── video_1e65a5cb_03.png
│   │   ├── video_1e65a5cb_03.txt
│   │   ├── video_1e65a5cb_04.png
│   │   ├── video_1e65a5cb_04.txt
│   │   ├── video_1e65a5cb_05.png
│   │   ├── video_1e65a5cb_05.txt
│   │   ├── video_1e65a5cb_06.png
│   │   ├── video_1e65a5cb_06.txt
│   │   ├── video_1e65a5cb_07.png
│   │   ├── video_1e65a5cb_07.txt
│   │   ├── video_1e65a5cb_08.png
│   │   ├── video_1e65a5cb_08.txt
│   │   ├── video_1e65a5cb_09.png
│   │   ├── video_1e65a5cb_09.txt
│   │   ├── video_1e65a5cb_10.png
│   │   ├── video_1e65a5cb_10.txt
│   │   ├── video_20bfada3_00.png
│   │   ├── video_20bfada3_00.txt
│   │   ├── video_20bfada3_01.png
│   │   ├── video_20bfada3_01.txt
│   │   ├── video_20bfada3_02.png
│   │   ├── video_20bfada3_02.txt
│   │   ├── video_20bfada3_03.png
│   │   ├── video_20bfada3_03.txt
│   │   ├── video_20bfada3_04.png
│   │   ├── video_20bfada3_04.txt
│   │   ├── video_20bfada3_05.png
│   │   ├── video_20bfada3_05.txt
│   │   ├── video_20bfada3_06.png
│   │   ├── video_20bfada3_06.txt
│   │   ├── video_20bfada3_07.png
│   │   ├── video_20bfada3_07.txt
│   │   ├── video_20bfada3_08.png
│   │   ├── video_20bfada3_08.txt
│   │   ├── video_26419b97_00.jpg
│   │   ├── video_26419b97_00.png
│   │   ├── video_26419b97_00.txt
│   │   ├── video_26419b97_01.jpg
│   │   ├── video_26419b97_01.png
│   │   ├── video_26419b97_01.txt
│   │   ├── video_26419b97_02.jpg
│   │   ├── video_26419b97_02.png
│   │   ├── video_26419b97_02.txt
│   │   ├── video_26419b97_03.jpg
│   │   ├── video_26419b97_03.png
│   │   ├── video_26419b97_03.txt
│   │   ├── video_26419b97_04.jpg
│   │   ├── video_26419b97_04.png
│   │   ├── video_26419b97_04.txt
│   │   ├── video_26419b97_05.jpg
│   │   ├── video_26419b97_05.png
│   │   ├── video_26419b97_05.txt
│   │   ├── video_26419b97_06.jpg
│   │   ├── video_26419b97_06.png
│   │   ├── video_26419b97_06.txt
│   │   ├── video_26419b97_07.jpg
│   │   ├── video_26419b97_07.png
│   │   ├── video_26419b97_07.txt
│   │   ├── video_26419b97_08.jpg
│   │   ├── video_26419b97_08.png
│   │   ├── video_26419b97_08.txt
│   │   ├── video_47c35852_00.png
│   │   ├── video_47c35852_00.txt
│   │   ├── video_47c35852_01.png
│   │   ├── video_47c35852_01.txt
│   │   ├── video_47c35852_02.png
│   │   ├── video_47c35852_02.txt
│   │   ├── video_47c35852_03.png
│   │   ├── video_47c35852_03.txt
│   │   ├── video_47c35852_04.png
│   │   ├── video_47c35852_04.txt
│   │   ├── video_47c35852_05.png
│   │   ├── video_47c35852_05.txt
│   │   ├── video_47c35852_06.png
│   │   ├── video_47c35852_06.txt
│   │   ├── video_47c35852_07.png
│   │   ├── video_47c35852_07.txt
│   │   ├── video_47c35852_08.png
│   │   ├── video_47c35852_08.txt
│   │   ├── video_5557ff52_00.png
│   │   ├── video_5557ff52_00.txt
│   │   ├── video_5557ff52_01.png
│   │   ├── video_5557ff52_01.txt
│   │   ├── video_5557ff52_02.png
│   │   ├── video_5557ff52_02.txt
│   │   ├── video_5557ff52_03.png
│   │   ├── video_5557ff52_03.txt
│   │   ├── video_5557ff52_04.png
│   │   ├── video_5557ff52_04.txt
│   │   ├── video_5557ff52_05.png
│   │   ├── video_5557ff52_05.txt
│   │   ├── video_5557ff52_06.png
│   │   ├── video_5557ff52_06.txt
│   │   ├── video_5557ff52_07.png
│   │   ├── video_5557ff52_07.txt
│   │   ├── video_60c569da_00.png
│   │   ├── video_60c569da_00.txt
│   │   ├── video_60c569da_01.png
│   │   ├── video_60c569da_01.txt
│   │   ├── video_60c569da_02.png
│   │   ├── video_60c569da_02.txt
│   │   ├── video_60c569da_03.png
│   │   ├── video_60c569da_03.txt
│   │   ├── video_60c569da_04.png
│   │   ├── video_60c569da_04.txt
│   │   ├── video_60c569da_05.png
│   │   ├── video_60c569da_05.txt
│   │   ├── video_60c569da_06.png
│   │   ├── video_60c569da_06.txt
│   │   ├── video_60c569da_07.png
│   │   ├── video_60c569da_07.txt
│   │   ├── video_60c569da_08.png
│   │   ├── video_60c569da_08.txt
│   │   ├── video_60c569da_09.png
│   │   ├── video_60c569da_09.txt
│   │   ├── video_6ccbc2d8_00.png
│   │   ├── video_6ccbc2d8_00.txt
│   │   ├── video_6ccbc2d8_01.png
│   │   ├── video_6ccbc2d8_01.txt
│   │   ├── video_6ccbc2d8_02.png
│   │   ├── video_6ccbc2d8_02.txt
│   │   ├── video_6ccbc2d8_03.png
│   │   ├── video_6ccbc2d8_03.txt
│   │   ├── video_6ccbc2d8_04.png
│   │   ├── video_6ccbc2d8_04.txt
│   │   ├── video_6ccbc2d8_05.png
│   │   ├── video_6ccbc2d8_05.txt
│   │   ├── video_6ccbc2d8_06.png
│   │   ├── video_6ccbc2d8_06.txt
│   │   ├── video_6ccbc2d8_07.png
│   │   ├── video_6ccbc2d8_07.txt
│   │   ├── video_6ccbc2d8_08.png
│   │   ├── video_6ccbc2d8_08.txt
│   │   ├── video_6ccbc2d8_09.png
│   │   ├── video_6ccbc2d8_09.txt
│   │   ├── video_6ccbc2d8_10.png
│   │   ├── video_6ccbc2d8_10.txt
│   │   ├── video_6ccbc2d8_11.png
│   │   ├── video_6ccbc2d8_11.txt
│   │   ├── video_6ed7d5de_00.jpg
│   │   ├── video_6ed7d5de_00.png
│   │   ├── video_6ed7d5de_00.txt
│   │   ├── video_6ed7d5de_01.jpg
│   │   ├── video_6ed7d5de_01.png
│   │   ├── video_6ed7d5de_01.txt
│   │   ├── video_6ed7d5de_02.jpg
│   │   ├── video_6ed7d5de_02.png
│   │   ├── video_6ed7d5de_02.txt
│   │   ├── video_6ed7d5de_03.jpg
│   │   ├── video_6ed7d5de_03.png
│   │   ├── video_6ed7d5de_03.txt
│   │   ├── video_6ed7d5de_04.jpg
│   │   ├── video_6ed7d5de_04.png
│   │   ├── video_6ed7d5de_04.txt
│   │   ├── video_6ed7d5de_05.jpg
│   │   ├── video_6ed7d5de_05.png
│   │   ├── video_6ed7d5de_05.txt
│   │   ├── video_6ed7d5de_06.jpg
│   │   ├── video_6ed7d5de_06.png
│   │   ├── video_6ed7d5de_06.txt
│   │   ├── video_6ed7d5de_07.jpg
│   │   ├── video_6ed7d5de_07.png
│   │   ├── video_6ed7d5de_07.txt
│   │   ├── video_8bfae113_00.png
│   │   ├── video_8bfae113_00.txt
│   │   ├── video_8bfae113_01.png
│   │   ├── video_8bfae113_01.txt
│   │   ├── video_8bfae113_02.png
│   │   ├── video_8bfae113_02.txt
│   │   ├── video_8bfae113_03.png
│   │   ├── video_8bfae113_03.txt
│   │   ├── video_8bfae113_04.png
│   │   ├── video_8bfae113_04.txt
│   │   ├── video_8bfae113_05.png
│   │   ├── video_8bfae113_05.txt
│   │   ├── video_8bfae113_06.png
│   │   ├── video_8bfae113_06.txt
│   │   ├── video_8bfae113_07.png
│   │   ├── video_8bfae113_07.txt
│   │   ├── video_8bfae113_08.png
│   │   ├── video_8bfae113_08.txt
│   │   ├── video_8bfae113_09.png
│   │   ├── video_8bfae113_09.txt
│   │   ├── video_94e49660_00.jpg
│   │   ├── video_94e49660_00.png
│   │   ├── video_94e49660_00.txt
│   │   ├── video_94e49660_01.jpg
│   │   ├── video_94e49660_01.png
│   │   ├── video_94e49660_01.txt
│   │   ├── video_94e49660_02.jpg
│   │   ├── video_94e49660_02.png
│   │   ├── video_94e49660_02.txt
│   │   ├── video_94e49660_03.jpg
│   │   ├── video_94e49660_03.png
│   │   ├── video_94e49660_03.txt
│   │   ├── video_94e49660_04.jpg
│   │   ├── video_94e49660_04.png
│   │   ├── video_94e49660_04.txt
│   │   ├── video_94e49660_05.jpg
│   │   ├── video_94e49660_05.png
│   │   ├── video_94e49660_05.txt
│   │   ├── video_94e49660_06.jpg
│   │   ├── video_94e49660_06.png
│   │   ├── video_94e49660_06.txt
│   │   ├── video_94e49660_07.jpg
│   │   ├── video_94e49660_07.png
│   │   ├── video_94e49660_07.txt
│   │   ├── video_acb73b49_00.png
│   │   ├── video_acb73b49_00.txt
│   │   ├── video_af830b6b_00.png
│   │   ├── video_af830b6b_00.txt
│   │   ├── video_af830b6b_01.png
│   │   ├── video_af830b6b_01.txt
│   │   ├── video_af830b6b_02.png
│   │   ├── video_af830b6b_02.txt
│   │   ├── video_af830b6b_03.png
│   │   ├── video_af830b6b_03.txt
│   │   ├── video_af830b6b_04.png
│   │   ├── video_af830b6b_04.txt
│   │   ├── video_af830b6b_05.png
│   │   ├── video_af830b6b_05.txt
│   │   ├── video_af830b6b_06.png
│   │   ├── video_af830b6b_06.txt
│   │   ├── video_af830b6b_07.png
│   │   ├── video_af830b6b_07.txt
│   │   ├── video_af830b6b_08.png
│   │   ├── video_af830b6b_08.txt
│   │   ├── video_af830b6b_09.png
│   │   ├── video_af830b6b_09.txt
│   │   ├── video_af830b6b_10.png
│   │   ├── video_af830b6b_10.txt
│   │   ├── video_af830b6b_11.png
│   │   ├── video_af830b6b_11.txt
│   │   ├── video_af830b6b_12.png
│   │   ├── video_af830b6b_12.txt
│   │   ├── video_af830b6b_13.png
│   │   ├── video_af830b6b_13.txt
│   │   ├── video_ba9eab3c_00.png
│   │   ├── video_ba9eab3c_00.txt
│   │   ├── video_ba9eab3c_01.png
│   │   ├── video_ba9eab3c_01.txt
│   │   ├── video_ba9eab3c_02.png
│   │   ├── video_ba9eab3c_02.txt
│   │   ├── video_ba9eab3c_03.png
│   │   ├── video_ba9eab3c_03.txt
│   │   ├── video_ba9eab3c_04.png
│   │   ├── video_ba9eab3c_04.txt
│   │   ├── video_ba9eab3c_05.png
│   │   ├── video_ba9eab3c_05.txt
│   │   ├── video_ba9eab3c_06.png
│   │   ├── video_ba9eab3c_06.txt
│   │   ├── video_ba9eab3c_07.png
│   │   ├── video_ba9eab3c_07.txt
│   │   ├── video_ba9eab3c_08.png
│   │   ├── video_ba9eab3c_08.txt
│   │   ├── video_ba9eab3c_09.png
│   │   ├── video_ba9eab3c_09.txt
│   │   ├── video_ba9eab3c_10.png
│   │   ├── video_ba9eab3c_10.txt
│   │   ├── video_ba9eab3c_11.png
│   │   ├── video_ba9eab3c_11.txt
│   │   ├── video_e8985306_00.png
│   │   ├── video_e8985306_00.txt
│   │   ├── video_e8985306_01.png
│   │   ├── video_e8985306_01.txt
│   │   ├── video_e8985306_02.png
│   │   ├── video_e8985306_02.txt
│   │   ├── video_e8985306_03.png
│   │   ├── video_e8985306_03.txt
│   │   ├── video_e8985306_04.png
│   │   ├── video_e8985306_04.txt
│   │   ├── video_e8985306_05.png
│   │   ├── video_e8985306_05.txt
│   │   ├── video_e8985306_06.png
│   │   ├── video_e8985306_06.txt
│   │   ├── video_e8985306_07.png
│   │   ├── video_e8985306_07.txt
│   │   ├── video_e8985306_08.png
│   │   ├── video_e8985306_08.txt
│   │   ├── video_e8985306_09.png
│   │   ├── video_e8985306_09.txt
│   │   ├── video_e8985306_10.png
│   │   ├── video_e8985306_10.txt
│   │   ├── video_e8985306_11.png
│   │   ├── video_e8985306_11.txt
│   │   ├── video_e992abb7_00.png
│   │   ├── video_e992abb7_00.txt
│   │   ├── video_e992abb7_01.png
│   │   ├── video_e992abb7_01.txt
│   │   ├── video_e992abb7_02.png
│   │   ├── video_e992abb7_02.txt
│   │   ├── video_e992abb7_03.png
│   │   ├── video_e992abb7_03.txt
│   │   ├── video_e992abb7_04.png
│   │   ├── video_e992abb7_04.txt
│   │   ├── video_e992abb7_05.png
│   │   ├── video_e992abb7_05.txt
│   │   ├── video_e992abb7_06.png
│   │   ├── video_e992abb7_06.txt
│   │   ├── video_e992abb7_07.png
│   │   ├── video_e992abb7_07.txt
│   │   ├── video_e992abb7_08.png
│   │   └── video_e992abb7_08.txt
│   ├── lora_weights
│   ├── mirrors
│   │   ├── 24b0fadc-831b-4382-8ba0-3387bf3f6891.png
│   │   ├── 7327de17-c64d-4fd2-ac95-9e748af3ddce.png
│   │   ├── b627aeee-892b-4d4e-b1b8-a344a88f0927.png
│   │   ├── bd3c7b18-e959-4468-9c74-3308a8111d3e.png
│   │   ├── c7f343db-e80d-4d51-a5bb-3c4b7ae5eec0.png
│   │   └── d2309ad7-8907-443a-a40d-96d332e5cf5f.png
│   ├── og-dataset-round-two.jpg
│   ├── streams
│   │   ├── 0044ebb3-e2c4-4eb6-b32e-36288e335ea4.txt
│   │   ├── 02e48816-a9ed-4cfe-9eaf-9007efc4106b.txt
│   │   ├── 04687ec8-d819-4753-a7dc-2031510e1b4f.txt
│   │   ├── 072c304a-f2d5-4163-b850-b71bfdfb8166.txt
│   │   ├── 07c7a056-70a2-4ede-b243-2f9d19a808af.txt
│   │   ├── 10e6bd6b-5394-4fe7-96b8-40eb7f1be72c.txt
│   │   ├── 114eafc2-4903-4369-beee-a8ed4df04281.txt
│   │   ├── 1201672e-4398-4839-b866-4c4500d37238.txt
│   │   ├── 1211497b-7238-46ee-9f2d-26ca2df31ba4.txt
│   │   ├── 144925b9-e140-4582-827f-68da5c689303.txt
│   │   ├── 14aa7171-784c-43a1-81ca-b8c3509f5f49.txt
│   │   ├── 1671b5f9-da7e-4511-9504-9ef539dec517.txt
│   │   ├── 1c5855bc-2c36-48cf-a333-80336180469c.txt
│   │   ├── 2123cda8-e051-435e-ab3d-7e5628b4637d.txt
│   │   ├── 2a625a81-9e7f-4b67-b344-2b13e953d433.txt
│   │   ├── 40b4117e-1a02-44f2-b6cc-9880cfcc8ee7.txt
│   │   ├── 44bfcb45-fd43-4896-878d-7af85b31c0b8.txt
│   │   ├── 475c6b3f-1ec0-4bbc-82c3-057efcc6e3a6.txt
│   │   ├── 49c8cf22-f085-46e7-8cb2-cbc71593c1e4.txt
│   │   ├── 4a3f8634-8d15-44a4-8878-a4a20f432921.txt
│   │   ├── 4dc51a1c-2b50-4e5f-9eb1-2decb4dd866b.txt
│   │   ├── 56ad3927-5812-42ff-8dc6-164e5fafbb3e.txt
│   │   ├── 580431ca-a80f-4be3-ba34-e27f92c1274f.txt
│   │   ├── 5897a445-64f3-4a82-a576-4536de5acd58.txt
│   │   ├── 5cff76d4-7fe9-4038-b008-49557e62daa5.txt
│   │   ├── 5e2973e2-b8c6-4409-89db-3491e6acf8ee.txt
│   │   ├── 68a110e9-86b1-417e-8d8a-00d9f85e6695.txt
│   │   ├── 7adde220-450b-42e0-a7dc-beb121102875.txt
│   │   ├── 7de3f962-8942-41e0-8507-c8b461bc0c39.txt
│   │   ├── 8083867d-0f24-4679-9506-b30954b49632.txt
│   │   ├── 87751aaa-76f2-42ad-9898-453e69cc30cc.txt
│   │   ├── 88d60caf-0556-462c-8d06-f2e11d5984aa.txt
│   │   ├── 9443768d-7d0a-4f0f-82a1-abdfb0a87401.txt
│   │   ├── 98af4573-a4fd-4fc2-a48e-f88c92a3925c.txt
│   │   ├── 9a222f6e-c2a3-4bb5-8195-46594dae6a36.txt
│   │   ├── 9a58523e-30e8-42be-8378-36b55ba51922.txt
│   │   ├── 9ccd3c09-4b04-415d-bf53-6ed37f1423e5.txt
│   │   ├── 9feb4c17-d346-4bb0-aa2b-c31466382c8c.txt
│   │   ├── a0f135a2-736d-45b2-b600-50a58d13baa7.txt
│   │   ├── a2bdda5f-78a5-4668-9cff-760c848e64ec.txt
│   │   ├── a5df0d11-63aa-4aa0-ba29-c142915c9f53.txt
│   │   ├── a7a3752f-6dfb-472a-8c5d-c724a8a0e1c6.txt
│   │   ├── b59416fd-6302-491a-9bbc-4efceb7cf712.txt
│   │   ├── bb0ae9a3-2515-4d6b-9f2c-cf2f80126a11.txt
│   │   ├── bb91048a-3d45-46a0-beea-060bcf0ca692.txt
│   │   ├── bc267744-4683-4ac9-9004-87b78bb785f9.txt
│   │   ├── cc39b176-72e2-4440-8be7-3fcfbde8fbe6.txt
│   │   ├── d19a4836-b473-4c5f-8c2a-6db6c35b6eb3.txt
│   │   ├── d20b7be3-2105-4fcc-9cc6-f822eeb6bf01.txt
│   │   ├── db477f9b-da17-4436-ac9d-a52b66095eaa.txt
│   │   ├── ddb31225-ef8e-474c-a1b5-3ecd2bbfe282.txt
│   │   ├── e1b6c0f9-5311-4d7a-836f-b4b833de1b56.txt
│   │   ├── e4f1cd4e-9989-40ad-a8eb-ed11ee21a262.txt
│   │   ├── e744a421-e474-4acf-a1b4-b9c431c3433e.txt
│   │   ├── e85c92c8-2667-4add-8eb1-f656dceac549.txt
│   │   ├── ed447366-3a03-422b-aa5c-0a616aa9d65c.txt
│   │   ├── f1f86725-40b3-4c4e-9b4e-8423c05602a3.txt
│   │   ├── f425aebc-cedb-4680-863c-c78234c672b0.txt
│   │   ├── f57cfdfe-bedc-4e20-b5c7-17e6272e0ed8.txt
│   │   └── fba82355-9fb5-446a-ac07-3eda93ee8a03.txt
│   ├── training-images
│   │   ├── 0243f958-5ab6-436e-a3ab-94cb0179e809.png
│   │   ├── 0243f958-5ab6-436e-a3ab-94cb0179e809.txt
│   │   ├── 02b3b56b-ff82-4a28-8aa7-be8a014aa705.png
│   │   ├── 02b3b56b-ff82-4a28-8aa7-be8a014aa705.txt
│   │   ├── 02c99c6f-92ef-4a44-8f10-62d54d817096.png
│   │   ├── 02c99c6f-92ef-4a44-8f10-62d54d817096.txt
│   │   ├── 05a561c7-a0da-45d9-95bd-17c7a6c60bb2.png
│   │   ├── 05a561c7-a0da-45d9-95bd-17c7a6c60bb2.txt
│   │   ├── 05b7ae07-547a-409b-b11f-9c6b3153e264.png
│   │   ├── 05b7ae07-547a-409b-b11f-9c6b3153e264.txt
│   │   ├── 06cdf0ff-52f9-4f29-9d18-36e2502744e2.png
│   │   ├── 06cdf0ff-52f9-4f29-9d18-36e2502744e2.txt
│   │   ├── 077f476e-75ee-421f-ac1c-6fe0a9522de2.png
│   │   ├── 077f476e-75ee-421f-ac1c-6fe0a9522de2.txt
│   │   ├── 07d0097d-83c4-4055-a089-cd1509073293.png
│   │   ├── 07d0097d-83c4-4055-a089-cd1509073293.txt
│   │   ├── 083cfe55-81a0-48d1-bd58-e49cb900f634.png
│   │   ├── 083cfe55-81a0-48d1-bd58-e49cb900f634.txt
│   │   ├── 08817708-b105-4f4b-8587-0c223ec78817.png
│   │   ├── 08817708-b105-4f4b-8587-0c223ec78817.txt
│   │   ├── 0f3b3b58-beea-4d39-9da6-db3ef0a043f8.png
│   │   ├── 0f3b3b58-beea-4d39-9da6-db3ef0a043f8.txt
│   │   ├── 101df410-0cec-45f5-8af7-f10e2897c516.png
│   │   ├── 101df410-0cec-45f5-8af7-f10e2897c516.txt
│   │   ├── 10f78f0a-fef1-424a-aac7-2706570caebd.png
│   │   ├── 10f78f0a-fef1-424a-aac7-2706570caebd.txt
│   │   ├── 1162ddcc-0c6e-4e63-80fe-5c1797641eb2.png
│   │   ├── 1162ddcc-0c6e-4e63-80fe-5c1797641eb2.txt
│   │   ├── 16aac48e-5fad-4446-9a09-c62af14410bf.png
│   │   ├── 16aac48e-5fad-4446-9a09-c62af14410bf.txt
│   │   ├── 18f7f9ba-1b3e-4b18-9d54-1b1e7fc3fca1.png
│   │   ├── 18f7f9ba-1b3e-4b18-9d54-1b1e7fc3fca1.txt
│   │   ├── 19000a17-820c-4d8f-933d-5992f30ee0b4.png
│   │   ├── 19000a17-820c-4d8f-933d-5992f30ee0b4.txt
│   │   ├── 1bb9ceb1-ea24-44b1-9742-4b6696ffef8d.png
│   │   ├── 1bb9ceb1-ea24-44b1-9742-4b6696ffef8d.txt
│   │   ├── 1cb86514-6d69-4b08-b50c-029e4c71aec2.png
│   │   ├── 1cb86514-6d69-4b08-b50c-029e4c71aec2.txt
│   │   ├── 1e31b141-2b47-469c-8c17-69dd24d51cc8.png
│   │   ├── 1e31b141-2b47-469c-8c17-69dd24d51cc8.txt
│   │   ├── 1e72bc3a-88c5-4a66-aa62-29e475fe848d.png
│   │   ├── 1e72bc3a-88c5-4a66-aa62-29e475fe848d.txt
│   │   ├── 1ff39b67-70b9-4a77-93b8-908e17d656e3.png
│   │   ├── 1ff39b67-70b9-4a77-93b8-908e17d656e3.txt
│   │   ├── 22dc4366-f2ab-44d0-bc96-740ae1ee4d1a.png
│   │   ├── 22dc4366-f2ab-44d0-bc96-740ae1ee4d1a.txt
│   │   ├── 2a928790-ee17-4f92-bbad-e4dfb4ba786d.png
│   │   ├── 2a928790-ee17-4f92-bbad-e4dfb4ba786d.txt
│   │   ├── 2d439e9f-3763-467a-8ede-88806941d881.png
│   │   ├── 2d439e9f-3763-467a-8ede-88806941d881.txt
│   │   ├── 3115c06a-a423-4c5a-a2f5-e0aaadc849ab.png
│   │   ├── 3115c06a-a423-4c5a-a2f5-e0aaadc849ab.txt
│   │   ├── 3119c586-9002-437f-b483-ec558b80a7cc.png
│   │   ├── 3119c586-9002-437f-b483-ec558b80a7cc.txt
│   │   ├── 35e29c9d-fd89-40d1-813e-49d02dbb8c90.png
│   │   ├── 35e29c9d-fd89-40d1-813e-49d02dbb8c90.txt
│   │   ├── 36415621-940c-4010-8f34-aff5aa012d42.png
│   │   ├── 36415621-940c-4010-8f34-aff5aa012d42.txt
│   │   ├── 36831230-20d4-4a21-a999-152c61feb268.png
│   │   ├── 36831230-20d4-4a21-a999-152c61feb268.txt
│   │   ├── 38632eda-bfea-44d5-b349-b7118c7401a8.png
│   │   ├── 38632eda-bfea-44d5-b349-b7118c7401a8.txt
│   │   ├── 38c53516-4bfe-4a3a-9935-9e0aea2cf43d.png
│   │   ├── 38c53516-4bfe-4a3a-9935-9e0aea2cf43d.txt
│   │   ├── 3a46ba7b-0d13-4440-92b3-bb90b1eef8e0.png
│   │   ├── 3a46ba7b-0d13-4440-92b3-bb90b1eef8e0.txt
│   │   ├── 3c069ab9-d14f-427b-8733-b0e6485adf61.png
│   │   ├── 3c069ab9-d14f-427b-8733-b0e6485adf61.txt
│   │   ├── 3d367d94-8f47-47a9-99cf-e9ce7bf6069c.png
│   │   ├── 3d367d94-8f47-47a9-99cf-e9ce7bf6069c.txt
│   │   ├── 3dd653cc-0e1d-4d60-9508-6b44b7052864.png
│   │   ├── 3dd653cc-0e1d-4d60-9508-6b44b7052864.txt
│   │   ├── 42dbc03d-43b8-4a39-9a60-1c74041d4c37.png
│   │   ├── 42dbc03d-43b8-4a39-9a60-1c74041d4c37.txt
│   │   ├── 46707a09-266b-4ac8-9c6b-991531e520df.png
│   │   ├── 46707a09-266b-4ac8-9c6b-991531e520df.txt
│   │   ├── 52013232-f524-4fd1-a1b4-b5a010f27db5.png
│   │   ├── 52013232-f524-4fd1-a1b4-b5a010f27db5.txt
│   │   ├── 5666069c-d519-41f4-8787-0dcc6c17a935.png
│   │   ├── 5666069c-d519-41f4-8787-0dcc6c17a935.txt
│   │   ├── 6160693c-33a8-4b8d-99f5-3d88e4cc571e.png
│   │   ├── 6160693c-33a8-4b8d-99f5-3d88e4cc571e.txt
│   │   ├── 631bc513-801b-4a33-8079-7bd0d978240c.png
│   │   ├── 631bc513-801b-4a33-8079-7bd0d978240c.txt
│   │   ├── 634d52f9-cb8a-4536-8ba4-fb786778a6dd.png
│   │   ├── 634d52f9-cb8a-4536-8ba4-fb786778a6dd.txt
│   │   ├── 688a8669-5bf8-4b78-8dd0-0044ae7ee0f7.png
│   │   ├── 688a8669-5bf8-4b78-8dd0-0044ae7ee0f7.txt
│   │   ├── 6980b6e2-7355-4ad4-b8d4-5735ac5eb467.png
│   │   ├── 6980b6e2-7355-4ad4-b8d4-5735ac5eb467.txt
│   │   ├── 6b72266e-5b60-40e2-acf2-fdb0a1f0f43b.png
│   │   ├── 6b72266e-5b60-40e2-acf2-fdb0a1f0f43b.txt
│   │   ├── 6e3506f0-388a-4b79-bb30-1aae0a735816.png
│   │   ├── 6e3506f0-388a-4b79-bb30-1aae0a735816.txt
│   │   ├── 6ee01240-e331-4418-a55c-b79ca468182a.png
│   │   ├── 6ee01240-e331-4418-a55c-b79ca468182a.txt
│   │   ├── 72cf09cc-4ed7-4e17-a362-4828617398fb.png
│   │   ├── 72cf09cc-4ed7-4e17-a362-4828617398fb.txt
│   │   ├── 7b08edcd-cdc4-4e6f-ab31-355886350390.png
│   │   ├── 7b08edcd-cdc4-4e6f-ab31-355886350390.txt
│   │   ├── 7be23c93-27c6-4521-a0b0-c7dd7ff8c47b.png
│   │   ├── 7be23c93-27c6-4521-a0b0-c7dd7ff8c47b.txt
│   │   ├── 7c7ad5fc-41d3-4864-93f9-e459a97d041a.png
│   │   ├── 7c7ad5fc-41d3-4864-93f9-e459a97d041a.txt
│   │   ├── 7dd4e67e-861c-46ec-a16b-79a65af7c08c.png
│   │   ├── 7dd4e67e-861c-46ec-a16b-79a65af7c08c.txt
│   │   ├── 81572138-55f9-467d-880c-6a62cfb3a0bd.png
│   │   ├── 81572138-55f9-467d-880c-6a62cfb3a0bd.txt
│   │   ├── 81615afe-eec8-48b7-9645-9373d23944d3.png
│   │   ├── 81615afe-eec8-48b7-9645-9373d23944d3.txt
│   │   ├── 81d869d4-22f4-4954-a249-b1c53a060d4d.png
│   │   ├── 81d869d4-22f4-4954-a249-b1c53a060d4d.txt
│   │   ├── 88e36ac0-3274-4118-8cd7-9b20ac0b7058.png
│   │   ├── 88e36ac0-3274-4118-8cd7-9b20ac0b7058.txt
│   │   ├── 89d34122-653b-4287-b0c2-e7ff1bb3d6f6.png
│   │   ├── 89d34122-653b-4287-b0c2-e7ff1bb3d6f6.txt
│   │   ├── 8aad5d15-d4e0-4a94-8e46-2f741a941080.png
│   │   ├── 8aad5d15-d4e0-4a94-8e46-2f741a941080.txt
│   │   ├── 8adb5a3e-8dd5-4c1b-886d-42a95c229335.png
│   │   ├── 8adb5a3e-8dd5-4c1b-886d-42a95c229335.txt
│   │   ├── 8c3eba00-78ac-4f8d-aa36-d779565a9128.png
│   │   ├── 8c3eba00-78ac-4f8d-aa36-d779565a9128.txt
│   │   ├── 8ca89491-16ea-42c0-a8ed-8c0701701082.png
│   │   ├── 8ca89491-16ea-42c0-a8ed-8c0701701082.txt
│   │   ├── 91d81602-8f50-49f1-bcc5-a6f04db99d99.png
│   │   ├── 91d81602-8f50-49f1-bcc5-a6f04db99d99.txt
│   │   ├── 92446073-50d6-4b3e-b3cf-7365ec75d12a.png
│   │   ├── 92446073-50d6-4b3e-b3cf-7365ec75d12a.txt
│   │   ├── 93fa35e9-e4d7-4eb2-84fd-a5f84820d62d.png
│   │   ├── 93fa35e9-e4d7-4eb2-84fd-a5f84820d62d.txt
│   │   ├── 96d744b2-835f-4eaf-b45e-5de7cc80a407.png
│   │   ├── 96d744b2-835f-4eaf-b45e-5de7cc80a407.txt
│   │   ├── 98064017-92c6-4394-9648-1f7ced4b1e4f.png
│   │   ├── 98064017-92c6-4394-9648-1f7ced4b1e4f.txt
│   │   ├── 993c2fcb-a77f-4031-9c13-dc6fb20deeee.png
│   │   ├── 993c2fcb-a77f-4031-9c13-dc6fb20deeee.txt
│   │   ├── 99d566a9-b8a9-4c29-aca4-bfdad2511329.png
│   │   ├── 99d566a9-b8a9-4c29-aca4-bfdad2511329.txt
│   │   ├── 9e014ec0-b782-452c-9268-5f53a0d7a08a.png
│   │   ├── 9e014ec0-b782-452c-9268-5f53a0d7a08a.txt
│   │   ├── 9e3cfdf6-57f0-41f1-859e-e2db3a871bce.png
│   │   ├── 9e3cfdf6-57f0-41f1-859e-e2db3a871bce.txt
│   │   ├── 9f2b7ead-ddf3-4943-867a-e465278ecb86.png
│   │   ├── 9f2b7ead-ddf3-4943-867a-e465278ecb86.txt
│   │   ├── a6f4b534-17f3-4a63-b499-aaecdf2cac78.png
│   │   ├── a6f4b534-17f3-4a63-b499-aaecdf2cac78.txt
│   │   ├── a74ec392-1c31-412e-9447-389896b82ac5.png
│   │   ├── a74ec392-1c31-412e-9447-389896b82ac5.txt
│   │   ├── a7bab964-5f2c-4f05-8b52-d5067700e00a.png
│   │   ├── a7bab964-5f2c-4f05-8b52-d5067700e00a.txt
│   │   ├── ac8d34c1-2b83-4e0c-9ccb-467a6913c5c3.png
│   │   ├── ac8d34c1-2b83-4e0c-9ccb-467a6913c5c3.txt
│   │   ├── ad9101a0-c982-44fd-811d-54b4dd9b1f41.png
│   │   ├── ad9101a0-c982-44fd-811d-54b4dd9b1f41.txt
│   │   ├── b0ce1111-7175-4423-a8b4-1140bc84d9a5.png
│   │   ├── b0ce1111-7175-4423-a8b4-1140bc84d9a5.txt
│   │   ├── b3e38269-848c-46b8-ac86-8911c364a42e.png
│   │   ├── b3e38269-848c-46b8-ac86-8911c364a42e.txt
│   │   ├── b535cfc0-dd5a-4e3e-8fab-899d0ed1cf93.png
│   │   ├── b535cfc0-dd5a-4e3e-8fab-899d0ed1cf93.txt
│   │   ├── b76a8778-3d33-4510-9bfa-3fc1c5875e2e.png
│   │   ├── b76a8778-3d33-4510-9bfa-3fc1c5875e2e.txt
│   │   ├── bc19ccf5-eaa1-4b89-860a-dd8986f897e5.png
│   │   ├── bc19ccf5-eaa1-4b89-860a-dd8986f897e5.txt
│   │   ├── bee3a191-29a1-4f3f-8b6c-3a44de57aa4e.png
│   │   ├── bee3a191-29a1-4f3f-8b6c-3a44de57aa4e.txt
│   │   ├── c35daf9a-6ab5-46be-938a-25618987b709.png
│   │   ├── c35daf9a-6ab5-46be-938a-25618987b709.txt
│   │   ├── c5fdaf4b-50c1-4f74-9a00-f347ec03d353.png
│   │   ├── c5fdaf4b-50c1-4f74-9a00-f347ec03d353.txt
│   │   ├── cd0a098a-2684-44bc-9343-486297d46b2e.png
│   │   ├── cd0a098a-2684-44bc-9343-486297d46b2e.txt
│   │   ├── ceed3936-0b60-4e90-88e3-f804e7e34e02.png
│   │   ├── ceed3936-0b60-4e90-88e3-f804e7e34e02.txt
│   │   ├── d19a78b0-237f-4768-a794-4612b8d3e907.png
│   │   ├── d19a78b0-237f-4768-a794-4612b8d3e907.txt
│   │   ├── d3d3629a-ae38-4292-a8a3-99adeda89cb5.png
│   │   ├── d3d3629a-ae38-4292-a8a3-99adeda89cb5.txt
│   │   ├── d42e3956-1c48-4d15-bc8f-c231eef27acd.png
│   │   ├── d42e3956-1c48-4d15-bc8f-c231eef27acd.txt
│   │   ├── d44355bc-9db3-432d-86a3-f6b6aad65923.png
│   │   ├── d44355bc-9db3-432d-86a3-f6b6aad65923.txt
│   │   ├── d6eda782-5fe3-48e6-98be-8d3be42e3e85.png
│   │   ├── d6eda782-5fe3-48e6-98be-8d3be42e3e85.txt
│   │   ├── d8444e7a-3ac9-4752-9b44-c1ee0150943a.png
│   │   ├── d8444e7a-3ac9-4752-9b44-c1ee0150943a.txt
│   │   ├── dccc3619-9c8a-4250-8523-9b9642538a12.png
│   │   ├── dccc3619-9c8a-4250-8523-9b9642538a12.txt
│   │   ├── e0d3c8f6-7250-4b35-bdcd-763c161df666.png
│   │   ├── e0d3c8f6-7250-4b35-bdcd-763c161df666.txt
│   │   ├── e425dbc5-6873-40af-b46f-dc5d13cbfbc0.png
│   │   ├── e425dbc5-6873-40af-b46f-dc5d13cbfbc0.txt
│   │   ├── e51e0d1d-f840-4a67-9bfc-b388a07903de.png
│   │   ├── e51e0d1d-f840-4a67-9bfc-b388a07903de.txt
│   │   ├── e605fac1-21d2-4313-bce8-1e5c8c9cb6fc.png
│   │   ├── e605fac1-21d2-4313-bce8-1e5c8c9cb6fc.txt
│   │   ├── ea943b23-b42c-4f02-925a-2d847e8e840f.png
│   │   ├── ea943b23-b42c-4f02-925a-2d847e8e840f.txt
│   │   ├── ebce7458-d1eb-44a9-969c-0fd88b784afb.png
│   │   ├── ebce7458-d1eb-44a9-969c-0fd88b784afb.txt
│   │   ├── ecbf45e4-dcf5-47ee-a1dd-4758ca2b8ed5.png
│   │   ├── ecbf45e4-dcf5-47ee-a1dd-4758ca2b8ed5.txt
│   │   ├── eda98675-bfe9-42ad-8e61-503534945c86.png
│   │   ├── eda98675-bfe9-42ad-8e61-503534945c86.txt
│   │   ├── ee6e74ab-4815-4c3c-b280-76b14aa2e060.png
│   │   ├── ee6e74ab-4815-4c3c-b280-76b14aa2e060.txt
│   │   ├── f1ef3c43-2614-48e8-9b1e-6c4332d47d5c.png
│   │   ├── f1ef3c43-2614-48e8-9b1e-6c4332d47d5c.txt
│   │   ├── f215265f-c9f0-4884-9b38-ebb4fa39dda0.png
│   │   ├── f215265f-c9f0-4884-9b38-ebb4fa39dda0.txt
│   │   ├── f3ebab9c-de3e-45dc-9bae-9888ae4ef8fb.png
│   │   ├── f3ebab9c-de3e-45dc-9bae-9888ae4ef8fb.txt
│   │   ├── f48d194e-1777-40b0-8a74-cb6770783bc3.png
│   │   ├── f48d194e-1777-40b0-8a74-cb6770783bc3.txt
│   │   ├── fa21b2e0-6a26-4e31-a654-13f70b65a17f.png
│   │   ├── fa21b2e0-6a26-4e31-a654-13f70b65a17f.txt
│   │   ├── fc85f235-8378-423b-ae19-b0140396c969.png
│   │   ├── fc85f235-8378-423b-ae19-b0140396c969.txt
│   │   ├── fdec320d-0d5d-4b6f-8fc9-7120a9d6ef45.png
│   │   ├── fdec320d-0d5d-4b6f-8fc9-7120a9d6ef45.txt
│   │   ├── fefebad8-3ae5-4ba0-981a-949eca820456.png
│   │   ├── fefebad8-3ae5-4ba0-981a-949eca820456.txt
│   │   ├── ff058a6c-79a6-4589-843f-aeecc47bfc3b.png
│   │   ├── ff058a6c-79a6-4589-843f-aeecc47bfc3b.txt
│   │   ├── rejected
│   │   └── thumbs
│   ├── training-live
│   │   └── state.json
│   ├── training_runs
│   │   ├── 091d49b5-2c15-4f5e-b044-1fc7d19ae10a
│   │   ├── 19b5a974-02a1-4fbb-95be-cd893ae6a5d4
│   │   ├── 1aa49562-0e36-430b-a47f-9bda7adabfec
│   │   ├── 1d4dd888-d6b1-4633-b03d-45a8bbe6816d
│   │   ├── 24942f84-04cf-41da-a3db-4920374be95e
│   │   ├── 279cf363-ac02-4046-ac9e-8ab679ed7a21
│   │   ├── 29fc8efa-0334-4a39-9828-8a7e7f5ceacd
│   │   ├── 2c134a51-2138-4fc9-afa1-44d2469e7b65
│   │   ├── 2ddcd4a5-ecb0-458a-bec5-7f0450d27872
│   │   ├── 2df7b2b6-7efc-4ee7-88cf-3a7ba41591a3
│   │   ├── 30785636-c550-42fa-82b7-51204c4b428b
│   │   ├── 32af83a0-878e-4d1b-851e-041cab40e96d
│   │   ├── 39e097e9-ce1d-4ed0-8b67-88c296dc0ae8
│   │   ├── 3c46194b-e666-4f12-9f3c-868cab439d84
│   │   ├── 3e3b9217-5d26-4b61-bc25-c5fe0df5072f
│   │   ├── 45b440fc-af22-4f29-b767-cdf5408823e1
│   │   ├── 4b50652c-250f-4baf-bdcf-825445e13e5c
│   │   ├── 4c16e18c-280a-468d-a3fd-a5dff2ed7e68
│   │   ├── 4d5985f8-3a20-4aa5-81bd-aa1f1e3c8937
│   │   ├── 52ed14e6-5ea4-4dd3-9df0-4705663bf15a
│   │   ├── 5c77844d-abed-4c4c-82c6-d30ec5067e01
│   │   ├── 613c8dd9-047d-4faf-b54a-b1df38c44233
│   │   ├── 61fbaad0-0f19-4db8-b77e-9ab79c2845cb
│   │   ├── 667144fb-f69f-45b6-a782-959382ed5be6
│   │   ├── 6b3bc93a-a3f1-43c2-afd9-18da2345b313
│   │   ├── 769dbdda-df25-4bdd-9597-57a86cde448e
│   │   ├── 8426a7ec-b1d8-4c6f-80ec-0877530aec65
│   │   ├── 847a77a4-7a2b-4eed-a6e8-ce4225ebf83b
│   │   ├── 8da59068-e711-4285-9494-dd0e1152a5a3
│   │   ├── 90f3c214-8bf1-462b-856f-542ee57f791e
│   │   ├── 911324ea-c026-4a8a-9bc0-81da16adda95
│   │   ├── 9d93ad00-f7ef-4538-ba08-81804f78596f
│   │   ├── 9dca5f2d-d0d1-42e4-aaec-1775c81f8453
│   │   ├── 9e05d7dc-072f-44ed-8dc8-ff7586ad2d91
│   │   ├── a1f8eeef-5851-42c7-b594-d269ccec87f1
│   │   ├── a8c3c135-9dcf-4803-9d64-7e7983a5dfd8
│   │   ├── ba421a1e-32cd-446c-a4a7-1075040fcc72
│   │   ├── bef0d906-8a2e-402d-b1a4-6849ee93077f
│   │   ├── c24fd39e-9fe0-4b8e-9f96-83329a95356a
│   │   ├── c405b010-39c5-47a5-9fef-c1a44772fbc4
│   │   ├── c7534141-00d6-4436-8a71-abb6825b6be1
│   │   ├── cac9f929-2bd9-449d-9d68-34bc6d5bc767
│   │   ├── cd7ada4a-0dc3-43cf-b064-05170f374f5e
│   │   ├── e56e37e6-8618-46c3-b607-f566253087c6
│   │   ├── e7e508e6-75c0-4b9d-926b-73a41e324877
│   │   ├── ee90c90d-b324-4e01-857a-f17285276fad
│   │   ├── eea1450d-2bae-400a-898c-deee7157ce23
│   │   ├── f27d981e-9edb-4c9c-937c-182e9d7ae299
│   │   ├── f29da2ce-8415-41c4-a7ac-233416e87936
│   │   ├── f4f501dd-01ae-481c-8e27-1d0f77f37eeb
│   │   ├── f6be4d0a-5e4d-4a5c-8b5f-53fd0cb67900
│   │   └── f8ac7f5d-c917-424a-9acc-941c88ccfb07
│   ├── v1_announcements
│   ├── videos
│   │   ├── 0f341deb-43c4-4285-9234-dcdeded40833
│   │   ├── 0f341deb-43c4-4285-9234-dcdeded40833.mp4
│   │   ├── 1a5e4365-7238-406c-8c78-488ee472b1f3
│   │   ├── 1a5e4365-7238-406c-8c78-488ee472b1f3.mp4
│   │   ├── 1e65a5cb-65f6-4904-a7c3-572efb4b258b
│   │   ├── 1e65a5cb-65f6-4904-a7c3-572efb4b258b.mp4
│   │   ├── 20bfada3-c243-47d7-b26e-a15054faaf9b
│   │   ├── 20bfada3-c243-47d7-b26e-a15054faaf9b.mp4
│   │   ├── 5557ff52-b256-410f-bb89-d764e47bf5fb
│   │   ├── 5557ff52-b256-410f-bb89-d764e47bf5fb.mp4
│   │   ├── 6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e
│   │   ├── 6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e.mp4
│   │   ├── 8bfae113-5e60-4bdc-af0b-3c5f5c806e47
│   │   ├── 8bfae113-5e60-4bdc-af0b-3c5f5c806e47.mp4
│   │   ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2
│   │   ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2.mp4
│   │   ├── ba9eab3c-230d-45c4-b0dd-ad4564073630
│   │   ├── ba9eab3c-230d-45c4-b0dd-ad4564073630.mp4
│   │   ├── create-video-bakery-opening-bell.mp4
│   │   ├── create-video-bedroom-journal.mp4
│   │   ├── create-video-bookstore-whisper.mp4
│   │   ├── create-video-boxing-gym-corner.mp4
│   │   ├── create-video-bus-stop-rain.mp4
│   │   ├── create-video-classroom-after-hours.mp4
│   │   ├── create-video-community-garden.mp4
│   │   ├── create-video-dance-studio.mp4
│   │   ├── create-video-diner-listen.mp4
│   │   ├── create-video-empty-apartment-first-night.mp4
│   │   ├── create-video-family-dinner.mp4
│   │   ├── create-video-fire-escape-voicemail.mp4
│   │   ├── create-video-grocery-aisle.mp4
│   │   ├── create-video-haircut-mirror.mp4
│   │   ├── create-video-hospital-corridor.mp4
│   │   ├── create-video-kitchen-tea.mp4
│   │   ├── create-video-laundromat-fold.mp4
│   │   ├── create-video-mechanic-garage.mp4
│   │   ├── create-video-office-stairwell.mp4
│   │   ├── create-video-park-bench-breakup.mp4
│   │   ├── create-video-recording-booth-first-true-take.mp4
│   │   ├── create-video-rooftop-sunrise.mp4
│   │   ├── create-video-seaside-walk.mp4
│   │   ├── create-video-shelter-adoption-moment.mp4
│   │   ├── create-video-sidewalk-chalk.mp4
│   │   ├── create-video-subway-window.mp4
│   │   ├── create-video-thrift-store-new-self.mp4
│   │   ├── create-video-wedding-speech-side-room.mp4
│   │   ├── e8985306-28e5-4655-b73e-e2d12c46837b
│   │   ├── e8985306-28e5-4655-b73e-e2d12c46837b.mp4
│   │   ├── e992abb7-c3b2-4d76-8253-df43ea9d171a
│   │   ├── e992abb7-c3b2-4d76-8253-df43ea9d171a.mp4
│   │   ├── mf-0bc22c83-5cb.mp4
│   │   ├── mf-34aa470b-bec.mp4
│   │   ├── mf-42ce79a1-082.mp4
│   │   ├── mf-703842a8-4c3.mp4
│   │   ├── mf-84794fab-cb0.mp4
│   │   ├── mf-896bd4d6-7b8.mp4
│   │   ├── mf-93937565-773.mp4
│   │   ├── mf-a816f2a0-3c0.mp4
│   │   ├── mf-a8b1abcd-81f.mp4
│   │   ├── mf-bd691add-62f.mp4
│   │   ├── mf-df637c54-463.mp4
│   │   └── mf-ec03cd00-212.mp4
│   └── writings
│       ├── 0x1eb8f497f6b6d70a1b6c2e241e5ac5317adc5040
│       ├── 0x7916707ea9984bcf31ed09df1876f694e9cf99c4
│       ├── 0x968210d94d7fadbcca306774b036ba7ec17b612a
│       ├── 0xada8e0625d9c7eccd1c5a9a7ac9fdd9756dbec33
│       └── 0xf9b6f131a088e6ef8daac7c09ce698f65d7eedc4
├── deploy
│   ├── README.md
│   ├── anky-heart.service
│   └── anky-mind.service
├── docs
│   ├── agents
│   │   ├── agi.mdx
│   │   ├── dumb.mdx
│   │   ├── overview.mdx
│   │   └── smart.mdx
│   ├── anky-x-presence.md
│   ├── api-reference
│   │   ├── authentication.mdx
│   │   ├── endpoints
│   │   └── introduction.mdx
│   ├── architecture
│   │   ├── overview.mdx
│   │   ├── pipelines.mdx
│   │   └── seed-identity.mdx
│   ├── build.js
│   ├── concepts
│   │   ├── ankycoin.mdx
│   │   ├── ankyverse.mdx
│   │   ├── chakras.mdx
│   │   ├── everything-is-an-excuse.mdx
│   │   ├── kingdoms.mdx
│   │   ├── sojourns.mdx
│   │   ├── stories.mdx
│   │   └── writing-practice.mdx
│   ├── images
│   ├── internal
│   │   ├── EXTENSION.md
│   │   ├── OPERATIONS.md
│   │   ├── marketing
│   │   ├── mobile-ios-now-seed-spec.md
│   │   ├── mobile-ios-three-tabs-spec.md
│   │   └── mobile-seed-identity.md
│   ├── introduction
│   │   ├── overview.mdx
│   │   ├── philosophy.mdx
│   │   └── quickstart.mdx
│   ├── logo
│   │   ├── anky-dark.svg
│   │   ├── anky-light.svg
│   │   └── favicon.svg
│   ├── marketing
│   │   ├── instagram_queue.json
│   │   └── instagram_queue_state.json
│   ├── orbiter.json
│   ├── package-lock.json
│   ├── package.json
│   └── self-hosting
│       ├── configuration.mdx
│       ├── deployment.mdx
│       └── requirements.mdx
├── extension
│   ├── background.js
│   ├── content.js
│   ├── icons
│   │   ├── icon128.png
│   │   ├── icon16.png
│   │   └── icon48.png
│   ├── manifest.json
│   ├── popup.html
│   ├── popup.js
│   └── styles.css
├── flux
│   ├── experiment-1
│   │   ├── 001.png
│   │   ├── 002.png
│   │   ├── 003.png
│   │   ├── 004.png
│   │   ├── 005.png
│   │   ├── 006.png
│   │   ├── 007.png
│   │   ├── 008.png
│   │   ├── 009.png
│   │   ├── 010.png
│   │   └── prompts.txt
│   └── experiment-2
│       ├── 001.png
│       ├── 002.png
│       ├── 003.png
│       ├── 004.png
│       ├── 005.png
│       ├── 006.png
│       ├── 007.png
│       ├── 008.png
│       ├── 009.png
│       ├── 010.png
│       ├── 011.png
│       ├── 012.png
│       ├── 013.png
│       ├── 014.png
│       ├── 015.png
│       ├── config.json
│       └── prompts.txt
├── interview-engine
│   ├── __pycache__
│   │   ├── audio.cpython-313.pyc
│   │   ├── brain.cpython-313.pyc
│   │   ├── compositor.cpython-313.pyc
│   │   └── memory.cpython-313.pyc
│   ├── assets
│   │   └── guest_default.png
│   ├── audio.py
│   ├── brain.py
│   ├── compositor.py
│   ├── memory.db
│   ├── memory.py
│   ├── models
│   │   ├── en_US-lessac-medium.onnx
│   │   └── en_US-lessac-medium.onnx.json
│   ├── requirements.txt
│   └── server.py
├── livestream
│   └── ep2
│       └── index.html
├── logs
│   ├── anky_dca.log
│   └── comfyui.log
├── ls
├── migrations
│   ├── 20260317_001_create_child_profiles.sql
│   ├── 20260317_002_create_cuentacuentos.sql
│   └── 20260317_003_create_cuentacuentos_images.sql
├── missfont.log
├── output.log
├── prompts
│   ├── 0001.md
│   ├── cuentacuentos_system.md
│   ├── generate_anky_soul_from_research.md
│   └── run_research_prompt.md
├── research_outputs
├── research_prompts
│   └── chakras_story_research.md
├── scripts
│   ├── __pycache__
│   │   ├── anky_dca_buy.cpython-313.pyc
│   │   ├── ankys_autopost.cpython-314.pyc
│   │   ├── autonomous_agent_v2.cpython-313.pyc
│   │   ├── autonomous_anky.cpython-313.pyc
│   │   ├── autonomous_anky.cpython-314.pyc
│   │   ├── autonomous_anky_poster.cpython-313.pyc
│   │   ├── autonomous_poster.cpython-314.pyc
│   │   ├── build_instagram_queue.cpython-313.pyc
│   │   ├── caption_missing_gallery_images.cpython-313.pyc
│   │   └── export_round_two_dataset.cpython-313.pyc
│   ├── anky_dca_buy.py
│   ├── ankys_autopost.py
│   ├── autonomous_agent_v2.py
│   ├── autonomous_anky.py
│   ├── autonomous_anky_poster.py
│   ├── autonomous_poster.py
│   ├── build_instagram_queue.py
│   ├── caption_missing_gallery_images.py
│   ├── carousel_gen_stdlib.py
│   ├── create_agent.py
│   ├── create_batch.py
│   ├── export_round_two_dataset.py
│   ├── generate_anky_day2.py
│   ├── generate_batch.py
│   ├── generate_landing_gifs.sh
│   ├── generate_pitch_deck.py
│   ├── generate_stories.py
│   ├── generate_training_images.py
│   ├── instagram_carousel_gen.py
│   ├── recaption_dataset.py
│   ├── run_anky_dca.sh
│   ├── run_autonomous_ankey.py
│   ├── test_flux.py
│   └── test_session_api.py
├── skills
│   └── colosseum-copilot -> ../.agents/skills/colosseum-copilot
├── skills-lock.json
├── skills.md
├── slides
│   ├── ep2
│   │   ├── anky-session.png
│   │   ├── index.html
│   │   └── tv.html
│   ├── index.html -> /home/kithkui/anky/slides/livestream-slides.html
│   └── livestream-slides.html
├── src
│   ├── ankyverse.rs
│   ├── config.rs
│   ├── create_videos.rs
│   ├── db
│   │   ├── migrations.rs
│   │   ├── mod.rs
│   │   └── queries.rs
│   ├── error.rs
│   ├── kingdoms.rs
│   ├── main.rs
│   ├── memory
│   │   ├── embeddings.rs
│   │   ├── extraction.rs
│   │   ├── mod.rs
│   │   ├── profile.rs
│   │   └── recall.rs
│   ├── middleware
│   │   ├── api_auth.rs
│   │   ├── honeypot.rs
│   │   ├── mod.rs
│   │   ├── security_headers.rs
│   │   ├── subdomain.rs
│   │   └── x402.rs
│   ├── models
│   │   ├── anky_story.rs
│   │   └── mod.rs
│   ├── pipeline
│   │   ├── collection.rs
│   │   ├── cost.rs
│   │   ├── guidance_gen.rs
│   │   ├── image_gen.rs
│   │   ├── memory_pipeline.rs
│   │   ├── mod.rs
│   │   ├── prompt_gen.rs
│   │   ├── stream_gen.rs
│   │   └── video_gen.rs
│   ├── public
│   │   ├── anky-1.png
│   │   ├── anky-2.png
│   │   └── anky-3.png
│   ├── routes
│   │   ├── api.rs
│   │   ├── auth.rs
│   │   ├── collection.rs
│   │   ├── dashboard.rs
│   │   ├── evolve.rs
│   │   ├── extension_api.rs
│   │   ├── generations.rs
│   │   ├── health.rs
│   │   ├── interview.rs
│   │   ├── live.rs
│   │   ├── mod.rs
│   │   ├── notification.rs
│   │   ├── pages.rs
│   │   ├── payment.rs
│   │   ├── payment_helper.rs
│   │   ├── poiesis.rs
│   │   ├── prompt.rs
│   │   ├── session.rs
│   │   ├── settings.rs
│   │   ├── simulations.rs
│   │   ├── social_context.rs
│   │   ├── swift.rs
│   │   ├── training.rs
│   │   ├── voices.rs
│   │   ├── webhook_farcaster.rs
│   │   ├── webhook_x.rs
│   │   └── writing.rs
│   ├── services
│   │   ├── apns.rs
│   │   ├── claude.rs
│   │   ├── comfyui.rs
│   │   ├── gemini.rs
│   │   ├── grok.rs
│   │   ├── hermes.rs
│   │   ├── honcho.rs
│   │   ├── mind.rs
│   │   ├── mod.rs
│   │   ├── neynar.rs
│   │   ├── notification.rs
│   │   ├── ollama.rs
│   │   ├── openrouter.rs
│   │   ├── payment.rs
│   │   ├── push_scheduler.rs
│   │   ├── r2.rs
│   │   ├── redis_queue.rs
│   │   ├── stream.rs
│   │   ├── tts.rs
│   │   ├── twitter.rs
│   │   ├── wallet.rs
│   │   └── x_bot.rs
│   ├── sse
│   │   ├── logger.rs
│   │   └── mod.rs
│   ├── state.rs
│   ├── storage
│   │   ├── files.rs
│   │   └── mod.rs
│   └── training
│       ├── dataset.rs
│       ├── mod.rs
│       ├── orchestrator.rs
│       ├── runner.rs
│       └── schedule.rs
├── static
│   ├── admin
│   │   ├── flux-lab.html
│   │   ├── media-factory.html
│   │   └── story-tester.html
│   ├── agent.json
│   ├── anky-data-part-aa
│   ├── anky-data-part-ab
│   ├── anky-data-part-ac
│   ├── anky-data-part-ad
│   ├── anky-data-part-ae
│   ├── anky-data-part-af
│   ├── anky-data-part-ag
│   ├── anky-data-part-ah
│   ├── anky-data-part-ai
│   ├── anky-data-part-aj
│   ├── anky-speech-square.mp4
│   ├── anky-speech-video.mp4
│   ├── anky-training-data.tar.gz
│   ├── ankycoin-farcaster.json
│   ├── apple-touch-icon.png
│   ├── autonomous
│   │   ├── 20260310_091533.png
│   │   ├── 20260311_210958.png
│   │   ├── 20260311_211312.png
│   │   ├── 20260312_123859_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
│   │   ├── 20260312_123859_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
│   │   ├── 20260312_123859_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
│   │   ├── 20260312_123859_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
│   │   ├── 20260312_124012_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
│   │   ├── 20260312_124012_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
│   │   ├── 20260312_124012_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
│   │   ├── 20260312_124012_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
│   │   ├── 20260312_124108_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
│   │   ├── 20260312_124108_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
│   │   ├── 20260312_124108_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
│   │   ├── 20260312_124108_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
│   │   ├── 20260312_124143_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
│   │   ├── 20260312_124143_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
│   │   ├── 20260312_124143_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
│   │   ├── 20260312_124143_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
│   │   ├── 20260312_124218_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
│   │   ├── 20260312_124218_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
│   │   ├── 20260312_124218_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
│   │   ├── 20260312_124218_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
│   │   ├── 20260312_124305_cbd512da-65ab-4ed6-9020-f94e7869f242.png
│   │   ├── 20260312_125809_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
│   │   ├── 20260312_125809_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
│   │   ├── 20260312_125809_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
│   │   ├── 20260312_125809_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
│   │   ├── 20260312_125817_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
│   │   ├── 20260312_125817_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
│   │   ├── 20260312_125817_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
│   │   ├── 20260312_125817_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
│   │   ├── 20260312_125949_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
│   │   ├── 20260312_125949_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
│   │   ├── 20260312_125949_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
│   │   ├── 20260312_125949_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
│   │   ├── 20260312_130131_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
│   │   ├── 20260312_130131_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
│   │   ├── 20260312_130131_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
│   │   ├── 20260312_130131_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
│   │   ├── 20260312_130141_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide01.jpg
│   │   ├── 20260312_130141_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide02.jpg
│   │   ├── 20260312_130141_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide03.jpg
│   │   ├── 20260312_130141_49ee1b49-ad74-477b-b211-ec0d646b9bd6_slide04.jpg
│   │   ├── 20260312_210316.png
│   │   ├── 20260313_210242.png
│   │   ├── 20260314_090321.png
│   │   ├── 20260314_210341.png
│   │   ├── 20260315_101651.png
│   │   ├── 20260315_212655.png
│   │   ├── 20260315_213041.png
│   │   ├── 20260316_104544.png
│   │   ├── 20260316_131312_cbd512da-65ab-4ed6-9020-f94e7869f242.png
│   │   ├── 20260316_131516_cbd512da-65ab-4ed6-9020-f94e7869f242.png
│   │   ├── 20260317_090230.png
│   │   ├── 20260318_105558.png
│   │   ├── 20260318_210348.png
│   │   ├── 20260318_210503_68893970.png
│   │   ├── 20260318_210526_68893970.png
│   │   ├── 20260320_142216_b8c7dc6d-153f-4c99-945b-025b9de841e1.png
│   │   ├── 20260320_143946_b8c7dc6d-153f-4c99-945b-025b9de841e1.png
│   │   ├── 20260320_144504_c1d73dbf-67a2-4b6f-a43c-9a9e250148bc.png
│   │   ├── 20260320_144959_cbd512da-65ab-4ed6-9020-f94e7869f242.png
│   │   ├── 20260320_145503_cbd512da-65ab-4ed6-9020-f94e7869f242.png
│   │   ├── 20260320_154836_9dd99459-1cc4-47f6-9b44-31e13656f6ca_slide01.jpg
│   │   ├── 20260320_154836_9dd99459-1cc4-47f6-9b44-31e13656f6ca_slide02.jpg
│   │   ├── 20260320_154836_9dd99459-1cc4-47f6-9b44-31e13656f6ca_slide03.jpg
│   │   ├── 20260320_154836_9dd99459-1cc4-47f6-9b44-31e13656f6ca_slide04.jpg
│   │   ├── 20260320_155448_9dd99459-1cc4-47f6-9b44-31e13656f6ca_slide01.jpg
│   │   ├── 20260320_155448_9dd99459-1cc4-47f6-9b44-31e13656f6ca_slide02.jpg
│   │   ├── 20260320_155448_9dd99459-1cc4-47f6-9b44-31e13656f6ca_slide03.jpg
│   │   ├── 20260320_155448_9dd99459-1cc4-47f6-9b44-31e13656f6ca_slide04.jpg
│   │   ├── 20260326_213330_anky.png
│   │   ├── 20260327_090609_anky.png
│   │   ├── 20260327_160156_anky.png
│   │   ├── 20260328_160144_anky.png
│   │   ├── anky_742_f04ab63e.json
│   │   └── x_post_742.txt
│   ├── changelog
│   │   ├── 2026-02-14-001-video-studio.txt
│   │   ├── 2026-02-14-002-paid-image-gen.txt
│   │   ├── 2026-02-14-003-x402-only.txt
│   │   ├── 2026-02-14-004-changelog.txt
│   │   ├── 2026-02-14-005-post-writing-ux.txt
│   │   ├── 2026-02-14-006-ux-overhaul.txt
│   │   ├── 2026-02-14-007-prompt-api-agents.txt
│   │   ├── 2026-02-15-001-remove-balance-payments.txt
│   │   ├── 2026-02-15-002-stream-overlay.txt
│   │   ├── 2026-02-15-003-livestream-overhaul.txt
│   │   ├── 2026-02-15-004-claim-username-modal.txt
│   │   ├── 2026-02-15-005-livestream-hardstop-congrats.txt
│   │   ├── 2026-02-15-006-write-rate-limit.txt
│   │   ├── 2026-02-15-007-farcaster-miniapp.txt
│   │   ├── 2026-02-15-008-farcaster-sdk-ready-images.txt
│   │   ├── 2026-02-15-009-bottom-live-bar-waiting-room.txt
│   │   ├── 2026-02-16-001-live-writing-ux-fixes.txt
│   │   ├── 2026-02-16-002-write-api-key-required.txt
│   │   ├── 2026-02-16-003-fix-anky-ca.txt
│   │   ├── 2026-02-16-004-livestream-watchdog.txt
│   │   ├── 2026-02-16-005-progressive-web-app.txt
│   │   ├── 2026-02-17-001-flow-score-leaderboard-chakra-pitch.txt
│   │   ├── 2026-02-17-002-no-cache-html-routes.txt
│   │   ├── 2026-02-17-003-video-pipeline-grok.txt
│   │   ├── 2026-02-17-004-fix-video-script-truncation.txt
│   │   ├── 2026-02-17-005-memory-enriched-video-pipeline.txt
│   │   ├── 2026-02-17-006-video-studio-filmstrip-parallel.txt
│   │   ├── 2026-02-18-001-video-pipeline-overhaul.txt
│   │   ├── 2026-02-18-002-vertical-video-continuity-cost.txt
│   │   ├── 2026-02-18-003-post-session-video-button.txt
│   │   ├── 2026-02-18-004-keyboard-first-feed-app.txt
│   │   ├── 2026-02-18-005-desktop-mobile-split.txt
│   │   ├── 2026-02-18-006-revert-keyboard-ui.txt
│   │   ├── 2026-02-19-001-vertical-story-driven-video.txt
│   │   ├── 2026-02-19-002-fix-chat-ui-anon-cookie.txt
│   │   ├── 2026-02-19-003-phantom-solana-login.txt
│   │   ├── 2026-02-19-004-email-social-login.txt
│   │   ├── 2026-02-19-005-infinite-media-slideshow.txt
│   │   ├── 2026-02-19-006-anky-tv-drawer-nav.txt
│   │   ├── 2026-02-19-007-video-playback-slideshow.txt
│   │   ├── 2026-02-19-008-feed-page.txt
│   │   ├── 2026-02-20-001-disable-livestream.txt
│   │   ├── 2026-02-20-002-fix-chat-textarea.txt
│   │   ├── 2026-02-20-003-memetics-wtf-homepage.txt
│   │   ├── 2026-02-21-001-meditation-first-experience.txt
│   │   ├── 2026-02-21-002-video-studio-mobile-ux.txt
│   │   ├── 2026-02-21-003-sequential-chain-video.txt
│   │   ├── 2026-02-21-004-psychoanalytic-director-prompt.txt
│   │   ├── 2026-02-21-005-dissolve-writing-friction.txt
│   │   ├── 2026-02-21-006-writing-first-homepage.txt
│   │   ├── 2026-02-21-007-fab-fix-inquiry-system.txt
│   │   ├── 2026-02-21-008-remove-meditation-fab.txt
│   │   ├── 2026-02-21-009-spanish-reflection-mobile-fix.txt
│   │   ├── 2026-02-21-010-settings-wallet-writings-scroll-video.txt
│   │   ├── 2026-02-22-001-fix-scroll-all-routes.txt
│   │   ├── 2026-02-25-001-fix-lost-writing-save-before-ollama.txt
│   │   ├── 2026-02-25-002-privy-auth-fallback-video-resilience.txt
│   │   ├── 2026-02-26-001-fix-video-payment-parallel-pipeline.txt
│   │   ├── 2026-02-26-002-anon-user-localstorage-persistence.txt
│   │   ├── 2026-02-26-003-suggested-replies-scroll-fix.txt
│   │   ├── 2026-02-26-004-interview-system-integration.txt
│   │   ├── 2026-02-26-005-live-interview-rename-reset.txt
│   │   ├── 2026-02-26-006-stt-timeout-protection.txt
│   │   ├── 2026-02-26-007-video-pipeline-story-spine.txt
│   │   ├── 2026-02-27-001-fix-video-payment-timeout.txt
│   │   ├── 2026-02-27-002-sharper-inquiry-prompts.txt
│   │   ├── 2026-02-27-003-structured-reflection-format.txt
│   │   ├── 2026-02-27-003-training-curation-tinder.txt
│   │   ├── 2026-02-27-004-kill-livestream.txt
│   │   ├── 2026-02-27-005-fix-torchaudio-training.txt
│   │   ├── 2026-02-28-001-trainings-journal.txt
│   │   ├── 2026-02-28-002-story-first-video-prompt.txt
│   │   ├── 2026-02-28-003-jpeg-images-for-xai-video.txt
│   │   ├── 2026-02-28-004-training-general-instructions.txt
│   │   ├── 2026-02-28-005-videos-gallery.txt
│   │   ├── 2026-02-28-006-fix-write-error-draft-loss.txt
│   │   ├── 2026-02-28-007-farcaster-miniapp-user-id-fallback.txt
│   │   ├── 2026-03-01-001-flux-lora-free-image-gen.txt
│   │   ├── 2026-03-01-002-flux-raw-prompt-anky-validation.txt
│   │   ├── 2026-03-01-003-flux-ux-thinker-fix-prompt-hint.txt
│   │   ├── 2026-03-02-001-anky-speech-video.txt
│   │   ├── 2026-03-03-001-fix-streaming-layout-and-reply-buttons.txt
│   │   ├── 2026-03-03-002-simplify-flux-anky-validation.txt
│   │   ├── 2026-03-03-003-anky-lora-recaption-and-dataset-gen.txt
│   │   ├── 2026-03-03-004-switch-to-qwen35-35b-moe.txt
│   │   ├── 2026-03-03-005-tinder-image-review-ui.txt
│   │   ├── 2026-03-03-006-training-run-2-dataset-pipeline.txt
│   │   ├── 2026-03-03-007-sharpen-reflection-prompts-ramana-jed.txt
│   │   ├── 2026-03-03-008-local-embeddings-nomic-embed-text.txt
│   │   ├── 2026-03-03-009-move-all-non-reflection-to-local-qwen.txt
│   │   ├── 2026-03-03-010-dataset-round-two-gallery.txt
│   │   ├── 2026-03-04-001-runpod-training-bootstrap-hardening.txt
│   │   ├── 2026-03-04-002-well-known-agent.txt
│   │   ├── 2026-03-04-003-round-two-runbook-and-v2-serving.txt
│   │   ├── 2026-03-04-004-switch-to-qwen3-5-35b.txt
│   │   ├── 2026-03-04-005-x-webhook-crc-comfyui-mention-reply.txt
│   │   ├── 2026-03-04-006-x-webhook-image-rate-limit.txt
│   │   ├── 2026-03-05-001-polling-loop-new-webhook-logic.txt
│   │   ├── 2026-03-05-002-webhook-log-viewer.txt
│   │   ├── 2026-03-05-003-x-filtered-stream.txt
│   │   ├── 2026-03-05-004-fix-stuck-reading-screen-remove-hackathon-banner.txt
│   │   ├── 2026-03-05-005-prefetch-memory-context.txt
│   │   ├── 2026-03-06-001-leaderboard-styling.txt
│   │   ├── 2026-03-06-002-remove-nav-links.txt
│   │   ├── 2026-03-06-003-x-reply-context.txt
│   │   ├── 2026-03-07-001-gemini-flux-fallback.txt
│   │   ├── 2026-03-07-002-swift-mobile-api.txt
│   │   ├── 2026-03-07-003-personalized-guidance-queue.txt
│   │   ├── 2026-03-07-004-facilitator-marketplace.txt
│   │   ├── 2026-03-07-005-swift-agent-brief-understanding-whitepaper.txt
│   │   ├── 2026-03-08-001-flux-lora-trigger-word.txt
│   │   ├── 2026-03-08-002-autoresearch-llm-pipeline.txt
│   │   ├── 2026-03-09-001-anky-reply-identity.txt
│   │   ├── 2026-03-09-002-x-tag-hermes-bridge.txt
│   │   ├── 2026-03-09-003-evolve-dashboard-deploy.txt
│   │   ├── 2026-03-09-004-evolve-trace-and-x-fixes.txt
│   │   ├── 2026-03-09-005-reflection-memory-skills-language.txt
│   │   ├── 2026-03-10-001-agent-native-skill-evolution.txt
│   │   ├── 2026-03-10-002-everything-free.txt
│   │   ├── 2026-03-10-003-farcaster-bot-integration.txt
│   │   ├── 2026-03-12-001-rescue-writing-ownership-bug.txt
│   │   ├── 2026-03-13-001-pitch-subdomain-pdf.txt
│   │   ├── 2026-03-13-002-prompt-background-image.txt
│   │   ├── 2026-03-13-003-prompt-link-and-formatted-writing.txt
│   │   ├── 2026-03-15-001-radical-writing-ux.txt
│   │   ├── 2026-03-16-001-simplify-paused-screen.txt
│   │   ├── 2026-03-17-001-simple-og-metadata.txt
│   │   ├── 2026-03-18-001-ankyverse-stories.txt
│   │   ├── 2026-03-18-002-unify-mobile-write-api.txt
│   │   ├── 2026-03-18-003-honcho-identity-modeling.txt
│   │   ├── 2026-03-19-001-system-summaries.txt
│   │   ├── 2026-03-19-002-remove-sadhana-meditation-breathwork-facilitators.txt
│   │   ├── 2026-03-19-003-mobile-next-prompt-you-device-token.txt
│   │   ├── 2026-03-19-004-web-seed-auth.txt
│   │   ├── 2026-03-19-005-mobile-web-design-system.txt
│   │   ├── 2026-03-19-006-match-story-language-to-writing.txt
│   │   ├── 2026-03-19-007-soul-document-story-pipeline.txt
│   │   ├── 2026-03-20-001-swap-story-to-local-qwen-gpu-priority-queue.txt
│   │   ├── 2026-03-20-002-ritual-lifecycle.txt
│   │   ├── 2026-03-20-003-anky-voices-backend.txt
│   │   ├── 2026-03-21-001-fix-mobile-api-gaps.txt
│   │   ├── 2026-03-21-002-push-notifications.txt
│   │   ├── 2026-03-21-002-tts-pipeline.txt
│   │   ├── 2026-03-22-001-social-reply-pipeline-context.txt
│   │   ├── 2026-03-22-002-anky-talks-back.txt
│   │   ├── 2026-03-22-003-minting-endpoints.txt
│   │   ├── 2026-03-23-001-fix-csp-writing-ux.txt
│   │   ├── 2026-03-24-001-thread-based-ux-redesign.txt
│   │   ├── 2026-03-24-002-landing-page-write-route-prompts.txt
│   │   ├── 2026-03-24-003-manifesto-route.txt
│   │   ├── 2026-03-24-004-simplify-writing-ux.txt
│   │   ├── 2026-03-24-005-timer-viewport-enter.txt
│   │   ├── 2026-03-24-006-idle-bar-pause-resume.txt
│   │   ├── 2026-03-24-007-enter-send-live-nudges.txt
│   │   ├── 2026-03-24-008-model-selector-settings.txt
│   │   ├── 2026-03-24-008-thread-splitting-profile-grid.txt
│   │   ├── 2026-03-24-009-remove-miniapp-uuid-prompts.txt
│   │   ├── 2026-03-25-001-chat-bubble-post-writing-ux.txt
│   │   ├── 2026-03-25-002-universal-links-prompt-endpoint.txt
│   │   ├── 2026-03-25-003-never-lose-reflection.txt
│   │   ├── 2026-03-26-001-fix-post-writing-ux-streaming.txt
│   │   ├── 2026-03-26-002-landing-inline-writing.txt
│   │   ├── 2026-03-26-003-flux-lab-batch-image-gen.txt
│   │   ├── 2026-03-26-003-flux-media-factory.txt
│   │   ├── 2026-03-26-003-replace-ollama-with-haiku.txt
│   │   ├── 2026-03-26-004-fix-reflection-streaming.txt
│   │   ├── 2026-03-26-004-ollama-to-cloud.txt
│   │   ├── 2026-03-27-001-fix-reflection-streaming-warm-context.txt
│   │   ├── 2026-03-27-002-chat-interface-post-writing.txt
│   │   ├── 2026-03-27-003-chat-navbar-privy-login.txt
│   │   ├── 2026-03-27-004-chat-first-privy-login-profile.txt
│   │   ├── 2026-03-27-004-fix-flux-aspect-ratio.txt
│   │   ├── 2026-03-27-005-stories-desktop-tap-zones.txt
│   │   ├── 2026-03-27-006-history-prompts-login-fix.txt
│   │   ├── 2026-03-27-007-ankycoin-landing-page.txt
│   │   ├── 2026-03-27-008-mirror-endpoint-ankycoin-miniapp.txt
│   │   ├── 2026-03-27-009-ankycoin-website-landing.txt
│   │   ├── 2026-03-27-010-ankycoin-image-generator.txt
│   │   ├── 2026-03-27-011-forge-first-mobile-optimize.txt
│   │   ├── 2026-03-27-012-mirror-gallery-fid-lookup.txt
│   │   ├── 2026-03-27-013-mirror-cache-chat.txt
│   │   ├── 2026-03-27-013-two-line-farcaster-replies.txt
│   │   ├── 2026-03-27-014-evolved-mirror-frame-image.txt
│   │   ├── 2026-03-27-014-miniapp-profile-page.txt
│   │   ├── 2026-03-28-001-anky-page-redesign-anky-mode.txt
│   │   ├── 2026-03-28-001-mirror-mint-nft-contract.txt
│   │   ├── 2026-03-28-002-openrouter-fallback-session-summary.txt
│   │   ├── 2026-03-28-003-r2-cdn-anky-story.txt
│   │   ├── 2026-03-29-001-farcaster-community-writing-prompts.txt
│   │   ├── 2026-03-29-002-programming-classes-smart-detection.txt
│   │   └── 2026-03-29-003-local-first-mind-kingdoms.txt
│   ├── create_videos_prompts.json
│   ├── cuentacuentos
│   ├── dca-bot
│   │   ├── __pycache__
│   │   ├── anky_dca_buy.py
│   │   ├── install.sh
│   │   ├── log_monitor.py
│   │   └── run_anky_dca.sh
│   ├── ep2
│   │   └── index.html
│   ├── ethers.umd.min.js
│   ├── farcaster.json
│   ├── fonts
│   │   └── Righteous-Regular.ttf
│   ├── hf
│   │   ├── anky-flux-lora-v1-readme.md
│   │   ├── anky-flux-lora-v2-readme.md
│   │   ├── download-samples.sh
│   │   └── upload-checkpoints.py
│   ├── htmx-sse.js
│   ├── htmx.min.js
│   ├── icon-192.png
│   ├── icon-512.png
│   ├── icon.png
│   ├── inference_server.py
│   ├── livestream-episode-2.html
│   ├── manifest.json
│   ├── mobile.css
│   ├── og-black.svg
│   ├── og-dataset-round-two.jpg
│   ├── og-pitch-deck.png
│   ├── pitch-deck.pdf
│   ├── pitch-images
│   │   ├── 12cc69de-9cb1-4ff0-ac04-a23fd50e02f0.jpg
│   │   ├── 1a877907-65ec-46ad-a744-1fc1390ee822.jpg
│   │   ├── 5666069c-d519-41f4-8787-0dcc6c17a935.jpg
│   │   ├── 72a11b6e-3a25-451c-977a-8d5c39dd78f0.jpg
│   │   ├── 821d5d32-dd04-4eb5-bb4e-b4f8f7bc01c5.jpg
│   │   ├── 8d49ffe9-616b-4b50-81cd-5e049d11db52.jpg
│   │   ├── 9dd99459-1cc4-47f6-9b44-31e13656f6ca.jpg
│   │   ├── cbd512da-65ab-4ed6-9020-f94e7869f242.jpg
│   │   ├── d5525129-55d7-4815-8e0a-7f911c736690.jpg
│   │   ├── ef481b14-9381-4c9e-a0f0-a27b8ffd1b96.jpg
│   │   └── fba2d4fe-7aba-44c6-ba82-fa5a4351fe68.jpg
│   ├── references
│   │   ├── anky-1.png
│   │   ├── anky-2.png
│   │   └── anky-3.png
│   ├── solana-agent-registry
│   │   ├── all_domains.json
│   │   ├── all_skills.json
│   │   └── index.html
│   ├── splash.png
│   ├── style.css
│   ├── sw.js
│   ├── train_anky_setup.sh
│   └── watcher.py
├── templates
│   ├── anky.html
│   ├── ankycoin.html
│   ├── ankycoin_landing.html
│   ├── base.html
│   ├── changelog.html
│   ├── class.html
│   ├── classes_index.html
│   ├── collection.html
│   ├── collection_progress.html
│   ├── create_videos.html
│   ├── dashboard.html
│   ├── dataset_round_two.html
│   ├── dca.html
│   ├── dca_bot_code.html
│   ├── evolve.html
│   ├── feed.html
│   ├── feedback.html
│   ├── gallery.html
│   ├── generate.html
│   ├── generations_dashboard.html
│   ├── generations_list.html
│   ├── generations_review.html
│   ├── generations_tinder.html
│   ├── help.html
│   ├── home.html
│   ├── interview.html
│   ├── landing.html
│   ├── leaderboard.html
│   ├── llm.html
│   ├── login.html
│   ├── media_dashboard.html
│   ├── mint.html
│   ├── mobile.html
│   ├── pitch-deck.html
│   ├── pitch.html
│   ├── poiesis.html
│   ├── poiesis_log.html
│   ├── prompt.html
│   ├── prompt_create.html
│   ├── prompt_new.html
│   ├── settings.html
│   ├── simulations.html
│   ├── sleeping.html
│   ├── stories.html
│   ├── stream_overlay.html
│   ├── test.html
│   ├── training.html
│   ├── training_general_instructions.html
│   ├── training_live.html
│   ├── training_run.html
│   ├── trainings.html
│   ├── video.html
│   ├── video_pipeline.html
│   ├── videos.html
│   ├── writing_response.html
│   ├── writings.html
│   └── you.html
├── test_comfy_local.py
├── test_local.py
├── test_parallel_fix.py
├── test_payload.json
├── tools
│   ├── ffmpeg-static
│   │   ├── ffmpeg-7.0.2-amd64-static
│   │   └── ffmpeg-release-amd64-static.tar.xz
│   └── ollama-override.conf
├── training
│   ├── autoresearch
│   │   ├── __pycache__
│   │   ├── data
│   │   ├── export_writings.py
│   │   ├── logs
│   │   ├── prepare.py
│   │   ├── program.md
│   │   ├── pyproject.toml
│   │   ├── run.log
│   │   ├── run_daily.sh
│   │   ├── save_results.py
│   │   ├── tokenizer
│   │   ├── train.py
│   │   ├── upstream
│   │   └── uv.lock
│   ├── prepare_dataset.py
│   ├── requirements.txt
│   ├── test_lora.py
│   └── train_flux_lora.py
├── twitter_oauth.py
├── twitter_oauth_v2.py
└── videos
    ├── 0f341deb-43c4-4285-9234-dcdeded40833.mp4
    ├── 0f341deb-43c4-4285-9234-dcdeded40833__scene_00.mp4
    ├── 0f341deb-43c4-4285-9234-dcdeded40833__scene_01.mp4
    ├── 0f341deb-43c4-4285-9234-dcdeded40833__scene_02.mp4
    ├── 0f341deb-43c4-4285-9234-dcdeded40833__scene_03.mp4
    ├── 0f341deb-43c4-4285-9234-dcdeded40833__scene_04.mp4
    ├── 0f341deb-43c4-4285-9234-dcdeded40833__scene_05.mp4
    ├── 0f341deb-43c4-4285-9234-dcdeded40833__scene_06.mp4
    ├── 0f341deb-43c4-4285-9234-dcdeded40833__scene_07.mp4
    ├── 0f341deb-43c4-4285-9234-dcdeded40833__scene_08.mp4
    ├── 0f341deb-43c4-4285-9234-dcdeded40833__scene_09.mp4
    ├── 0f341deb-43c4-4285-9234-dcdeded40833__scene_10.mp4
    ├── 0f341deb-43c4-4285-9234-dcdeded40833__scene_11.mp4
    ├── 1a5e4365-7238-406c-8c78-488ee472b1f3.mp4
    ├── 1a5e4365-7238-406c-8c78-488ee472b1f3__scene_00.mp4
    ├── 1a5e4365-7238-406c-8c78-488ee472b1f3__scene_01.mp4
    ├── 1a5e4365-7238-406c-8c78-488ee472b1f3__scene_02.mp4
    ├── 1a5e4365-7238-406c-8c78-488ee472b1f3__scene_03.mp4
    ├── 1a5e4365-7238-406c-8c78-488ee472b1f3__scene_04.mp4
    ├── 1a5e4365-7238-406c-8c78-488ee472b1f3__scene_05.mp4
    ├── 1a5e4365-7238-406c-8c78-488ee472b1f3__scene_06.mp4
    ├── 1a5e4365-7238-406c-8c78-488ee472b1f3__scene_07.mp4
    ├── 1a5e4365-7238-406c-8c78-488ee472b1f3__scene_08.mp4
    ├── 1a5e4365-7238-406c-8c78-488ee472b1f3__scene_09.mp4
    ├── 1e65a5cb-65f6-4904-a7c3-572efb4b258b.mp4
    ├── 1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_00.mp4
    ├── 1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_01.mp4
    ├── 1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_02.mp4
    ├── 1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_03.mp4
    ├── 1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_04.mp4
    ├── 1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_05.mp4
    ├── 1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_06.mp4
    ├── 1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_07.mp4
    ├── 1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_08.mp4
    ├── 1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_09.mp4
    ├── 1e65a5cb-65f6-4904-a7c3-572efb4b258b__scene_10.mp4
    ├── 20bfada3-c243-47d7-b26e-a15054faaf9b.mp4
    ├── 20bfada3-c243-47d7-b26e-a15054faaf9b__scene_00.mp4
    ├── 20bfada3-c243-47d7-b26e-a15054faaf9b__scene_01.mp4
    ├── 20bfada3-c243-47d7-b26e-a15054faaf9b__scene_02.mp4
    ├── 20bfada3-c243-47d7-b26e-a15054faaf9b__scene_03.mp4
    ├── 20bfada3-c243-47d7-b26e-a15054faaf9b__scene_04.mp4
    ├── 20bfada3-c243-47d7-b26e-a15054faaf9b__scene_05.mp4
    ├── 20bfada3-c243-47d7-b26e-a15054faaf9b__scene_07.mp4
    ├── 20bfada3-c243-47d7-b26e-a15054faaf9b__scene_08.mp4
    ├── 26419b97-de18-446c-a7b0-b7ebd74aeac6.mp4
    ├── 26419b97-de18-446c-a7b0-b7ebd74aeac6__concat.txt
    ├── 26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_00.mp4
    ├── 26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_01.mp4
    ├── 26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_02.mp4
    ├── 26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_03.mp4
    ├── 26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_04.mp4
    ├── 26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_05.mp4
    ├── 26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_06.mp4
    ├── 26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_07.mp4
    ├── 26419b97-de18-446c-a7b0-b7ebd74aeac6__scene_08.mp4
    ├── 47c35852-3186-434c-8436-00a9f1558069.mp4
    ├── 47c35852-3186-434c-8436-00a9f1558069__concat.txt
    ├── 47c35852-3186-434c-8436-00a9f1558069__scene_00.mp4
    ├── 47c35852-3186-434c-8436-00a9f1558069__scene_01.mp4
    ├── 47c35852-3186-434c-8436-00a9f1558069__scene_02.mp4
    ├── 47c35852-3186-434c-8436-00a9f1558069__scene_03.mp4
    ├── 47c35852-3186-434c-8436-00a9f1558069__scene_04.mp4
    ├── 47c35852-3186-434c-8436-00a9f1558069__scene_05.mp4
    ├── 47c35852-3186-434c-8436-00a9f1558069__scene_06.mp4
    ├── 47c35852-3186-434c-8436-00a9f1558069__scene_07.mp4
    ├── 47c35852-3186-434c-8436-00a9f1558069__scene_08.mp4
    ├── 5557ff52-b256-410f-bb89-d764e47bf5fb.mp4
    ├── 5557ff52-b256-410f-bb89-d764e47bf5fb__scene_00.mp4
    ├── 5557ff52-b256-410f-bb89-d764e47bf5fb__scene_01.mp4
    ├── 5557ff52-b256-410f-bb89-d764e47bf5fb__scene_02.mp4
    ├── 5557ff52-b256-410f-bb89-d764e47bf5fb__scene_03.mp4
    ├── 5557ff52-b256-410f-bb89-d764e47bf5fb__scene_04.mp4
    ├── 5557ff52-b256-410f-bb89-d764e47bf5fb__scene_05.mp4
    ├── 5557ff52-b256-410f-bb89-d764e47bf5fb__scene_06.mp4
    ├── 5557ff52-b256-410f-bb89-d764e47bf5fb__scene_07.mp4
    ├── 60c569da-3cfc-41ae-a29f-8f4d2b740c2b.mp4
    ├── 60c569da-3cfc-41ae-a29f-8f4d2b740c2b__concat.txt
    ├── 60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_00.mp4
    ├── 60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_01.mp4
    ├── 60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_02.mp4
    ├── 60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_03.mp4
    ├── 60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_04.mp4
    ├── 60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_05.mp4
    ├── 60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_06.mp4
    ├── 60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_07.mp4
    ├── 60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_08.mp4
    ├── 60c569da-3cfc-41ae-a29f-8f4d2b740c2b__scene_09.mp4
    ├── 6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e.mp4
    ├── 6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_00.mp4
    ├── 6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_01.mp4
    ├── 6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_02.mp4
    ├── 6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_03.mp4
    ├── 6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_04.mp4
    ├── 6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_05.mp4
    ├── 6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_06.mp4
    ├── 6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_07.mp4
    ├── 6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_08.mp4
    ├── 6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_09.mp4
    ├── 6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_10.mp4
    ├── 6ccbc2d8-f8be-4ada-8c81-b45dfa6c901e__scene_11.mp4
    ├── 6ed7d5de-94d4-4298-a65b-e939f5ba8971.mp4
    ├── 6ed7d5de-94d4-4298-a65b-e939f5ba8971__concat.txt
    ├── 6ed7d5de-94d4-4298-a65b-e939f5ba8971__scene_00.mp4
    ├── 6ed7d5de-94d4-4298-a65b-e939f5ba8971__scene_01.mp4
    ├── 6ed7d5de-94d4-4298-a65b-e939f5ba8971__scene_02.mp4
    ├── 6ed7d5de-94d4-4298-a65b-e939f5ba8971__scene_03.mp4
    ├── 6ed7d5de-94d4-4298-a65b-e939f5ba8971__scene_04.mp4
    ├── 6ed7d5de-94d4-4298-a65b-e939f5ba8971__scene_05.mp4
    ├── 6ed7d5de-94d4-4298-a65b-e939f5ba8971__scene_06.mp4
    ├── 6ed7d5de-94d4-4298-a65b-e939f5ba8971__scene_07.mp4
    ├── 8bfae113-5e60-4bdc-af0b-3c5f5c806e47.mp4
    ├── 8bfae113-5e60-4bdc-af0b-3c5f5c806e47__scene_00.mp4
    ├── 8bfae113-5e60-4bdc-af0b-3c5f5c806e47__scene_01.mp4
    ├── 8bfae113-5e60-4bdc-af0b-3c5f5c806e47__scene_03.mp4
    ├── 8bfae113-5e60-4bdc-af0b-3c5f5c806e47__scene_04.mp4
    ├── 8bfae113-5e60-4bdc-af0b-3c5f5c806e47__scene_06.mp4
    ├── 8bfae113-5e60-4bdc-af0b-3c5f5c806e47__scene_07.mp4
    ├── 8bfae113-5e60-4bdc-af0b-3c5f5c806e47__scene_08.mp4
    ├── 94e49660-2bb2-44a8-b814-20ac666a61cd.mp4
    ├── 94e49660-2bb2-44a8-b814-20ac666a61cd__concat.txt
    ├── 94e49660-2bb2-44a8-b814-20ac666a61cd__scene_00.mp4
    ├── 94e49660-2bb2-44a8-b814-20ac666a61cd__scene_01.mp4
    ├── 94e49660-2bb2-44a8-b814-20ac666a61cd__scene_02.mp4
    ├── 94e49660-2bb2-44a8-b814-20ac666a61cd__scene_03.mp4
    ├── 94e49660-2bb2-44a8-b814-20ac666a61cd__scene_04.mp4
    ├── 94e49660-2bb2-44a8-b814-20ac666a61cd__scene_05.mp4
    ├── 94e49660-2bb2-44a8-b814-20ac666a61cd__scene_06.mp4
    ├── 94e49660-2bb2-44a8-b814-20ac666a61cd__scene_07.mp4
    ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2.mp4
    ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2__final.mp4
    ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_00.mp4
    ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_01.mp4
    ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_02.mp4
    ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_03.mp4
    ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_04.mp4
    ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_05.mp4
    ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_06.mp4
    ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_07.mp4
    ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_08.mp4
    ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_09.mp4
    ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_10.mp4
    ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_11.mp4
    ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_12.mp4
    ├── af830b6b-bfc2-4b3e-b6d6-584773a489a2__scene_13.mp4
    ├── anky-speech
    │   ├── anky_speech.mp3
    │   ├── anky_speech.srt
    │   ├── anky_video.mp4
    │   ├── bg_video.mp4
    │   ├── concat_list.txt
    │   ├── loop_base.mp4
    │   └── make_video.py
    ├── ba9eab3c-230d-45c4-b0dd-ad4564073630.mp4
    ├── ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_00.mp4
    ├── ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_01.mp4
    ├── ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_02.mp4
    ├── ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_03.mp4
    ├── ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_04.mp4
    ├── ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_05.mp4
    ├── ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_06.mp4
    ├── ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_07.mp4
    ├── ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_08.mp4
    ├── ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_09.mp4
    ├── ba9eab3c-230d-45c4-b0dd-ad4564073630__scene_10.mp4
    ├── e8985306-28e5-4655-b73e-e2d12c46837b.mp4
    ├── e8985306-28e5-4655-b73e-e2d12c46837b__scene_00.mp4
    ├── e8985306-28e5-4655-b73e-e2d12c46837b__scene_01.mp4
    ├── e8985306-28e5-4655-b73e-e2d12c46837b__scene_02.mp4
    ├── e8985306-28e5-4655-b73e-e2d12c46837b__scene_03.mp4
    ├── e8985306-28e5-4655-b73e-e2d12c46837b__scene_04.mp4
    ├── e8985306-28e5-4655-b73e-e2d12c46837b__scene_05.mp4
    ├── e8985306-28e5-4655-b73e-e2d12c46837b__scene_06.mp4
    ├── e8985306-28e5-4655-b73e-e2d12c46837b__scene_07.mp4
    ├── e8985306-28e5-4655-b73e-e2d12c46837b__scene_08.mp4
    ├── e8985306-28e5-4655-b73e-e2d12c46837b__scene_09.mp4
    ├── e8985306-28e5-4655-b73e-e2d12c46837b__scene_10.mp4
    ├── e8985306-28e5-4655-b73e-e2d12c46837b__scene_11.mp4
    ├── e992abb7-c3b2-4d76-8253-df43ea9d171a.mp4
    ├── e992abb7-c3b2-4d76-8253-df43ea9d171a__scene_00.mp4
    ├── e992abb7-c3b2-4d76-8253-df43ea9d171a__scene_01.mp4
    ├── e992abb7-c3b2-4d76-8253-df43ea9d171a__scene_03.mp4
    ├── e992abb7-c3b2-4d76-8253-df43ea9d171a__scene_04.mp4
    ├── e992abb7-c3b2-4d76-8253-df43ea9d171a__scene_05.mp4
    ├── e992abb7-c3b2-4d76-8253-df43ea9d171a__scene_06.mp4
    ├── e992abb7-c3b2-4d76-8253-df43ea9d171a__scene_07.mp4
    └── e992abb7-c3b2-4d76-8253-df43ea9d171a__scene_08.mp4

185 directories, 2738 files
```

</details>

## 2. Core Writing Loop

Primary browser loop: `GET /write` renders `templates/home.html`, which runs a plain-JS writing session with `SESSION_DURATION = 480.0` seconds and `IDLE_TIMEOUT = 8.0` seconds.

### 2.1 Frontend capture + inactivity timer

| Step | File | Function / block | What happens |
| --- | --- | --- | --- |
| Render writing UI | `src/routes/pages.rs` | `write_page` | Loads prompt/user context, ensures the `anky_user_id` cookie, and renders `home.html`. |
| Start timer | `templates/home.html` | `beginSession()` | Creates a browser-side `sessionId`, starts the 8-minute countdown, and starts periodic checkpoint saves. |
| Capture keystrokes | `templates/home.html` | `writingArea.addEventListener('keydown', ...)` | Starts the session on the first printable character, blocks backspace/delete/enter, records keystroke deltas, and updates `lastInputAt`. |
| Mobile input fallback | `templates/home.html` | `writingArea.addEventListener('input', ...)` | Keeps the timer alive for soft-keyboard input paths where `keydown` is unreliable. |
| Enforce idle timeout | `templates/home.html` | `tick()` | Recomputes `idleElapsed` on every tick and calls `pauseSession()` once inactivity reaches 8 seconds. |
| Pause/freeze | `templates/home.html` | `pauseSession()` | Freezes timers, stops checkpoint intervals, persists a final checkpoint, and exposes the send CTA. |
| Submit | `templates/home.html` | `doSend()` -> `sendToAnky(text)` | Sends the completed writing payload to the backend and pivots the UI into reflection/result mode. |

Important constants in `templates/home.html`:
- `IDLE_TIMEOUT = 8.0`
- `IDLE_VISIBLE_AFTER = 3.0`
- `SESSION_DURATION = 480.0`
- `CHECKPOINT_INTERVAL = 30000`

### 2.2 Submission endpoint

| Step | File | Function / handler | What happens |
| --- | --- | --- | --- |
| Save in-progress draft | `src/routes/api.rs` | `save_checkpoint` | `POST /api/checkpoint` upserts a checkpoint row and ties it to the active session/user. |
| Prewarm memory | `src/routes/api.rs` | `warm_context` | `POST /api/warm-context` precomputes Honcho/local memory near minute 6 to reduce end-of-session latency. |
| Accept completed browser session | `src/routes/writing.rs` | `process_writing` | `POST /write` receives `text`, `duration`, `flow_score`, `session_id`, `session_token`, and `keystroke_deltas`, rate-limits the request, and writes the session into SQLite. |

### 2.3 Post-submission processing

There are two materially different paths.

#### Non-anky write (< full ritual)

| Step | File | Function / handler | What happens |
| --- | --- | --- | --- |
| Persist completed writing | `src/routes/writing.rs` | `process_writing` -> `db::queries::upsert_completed_writing_session_with_flow` | Saves the completed `writing_sessions` row first. |
| Immediate short response | `src/routes/writing.rs` | `process_writing` -> `services::claude::call_haiku` | Generates an immediate short reply for the browser response. |
| Background fuller response | `src/routes/writing.rs` | spawned `pipeline::guidance_gen::generate_anky_response` | Generates/stores `anky_response`, `anky_next_prompt`, and `anky_mood` in the background. |
| Browser polling | `templates/home.html` | `pollForAnkyPrompt(sessionId)` | Polls `GET /api/writing/{sessionId}/status` until response/mood/next-prompt fields are populated. |

#### Full anky ritual (8 minutes and enough words)

| Step | File | Function / handler | What happens |
| --- | --- | --- | --- |
| Persist completed writing | `src/routes/writing.rs` | `process_writing` -> `db::queries::upsert_completed_writing_session_with_flow` | Stores the finished `writing_sessions` row before generation work starts. |
| Create pending anky row | `src/routes/writing.rs` | `process_writing` -> `db::queries::insert_anky` | Inserts an `ankys` row with status `generating` and origin `written`. |
| Queue GPU work | `src/routes/writing.rs` | `process_writing` -> `state.gpu_queue.submit(GpuJob::AnkyImage)` | Pushes the image-generation job into the in-memory GPU queue. |
| GPU worker drain | `src/main.rs` | `gpu_job_worker` | Single async worker drains pro jobs first, then free jobs, and runs image/story generation serially. |
| Generate image + fallback reflection | `src/pipeline/image_gen.rs` | `generate_anky_from_writing` | Builds an image prompt, tries Gemini image generation first, falls back to ComfyUI, writes image assets, optionally uploads to R2, builds the `.anky` payload, and marks the `ankys` row complete. |
| Stream reflection | `src/routes/api.rs` | `stream_reflection` | `GET /api/stream-reflection/{ankyId}` streams the title/reflection over SSE, saving them into SQLite when complete. |
| Browser SSE consumer | `templates/home.html` | `streamReflection(ankyId)` | Opens `EventSource('/api/stream-reflection/' + ankyId)` and progressively renders the final reflection text. |

### 2.4 Storage + return path

Storage locations:
- SQLite: `data/anky.db`
  - `writing_sessions`: raw writing, duration, word count, flow score, browser response, next prompt, mood.
  - `ankys`: title, reflection, image paths, story payload, mint metadata, status.
- Local files:
  - `data/images/`: generated images and derivatives.
  - `data/writings/`: archived writing text files for certain users / backfills.
- Optional Cloudflare R2 uploads from `src/pipeline/image_gen.rs` through `src/services/r2.rs`.

How results return to the user:
- Non-anky browser sessions: immediate JSON from `POST /write`, then browser polling to `GET /api/writing/{sessionId}/status`.
- Anky browser sessions: immediate placeholder JSON from `POST /write`, then reflection via `GET /api/stream-reflection/{ankyId}`, and finally the canonical detail page at `GET /anky/{id}`.
- Mobile sessions: `POST /swift/v1/write` and `POST /swift/v2/write` use `swift::submit_writing_unified`, with `GET /swift/v2/writing/{sessionId}/status` for follow-up polling.
- Agent sessions: `POST /api/v1/session/start` + `POST /api/v1/session/chunk`, finalized by timeout or explicit completion, then fetched via `GET /api/v1/session/{id}/result`.

### 2.5 Alternate agent/mobile write loops

There is a second 8-second enforcement path for agent-style chunked writing:
- File: `src/routes/session.rs`
- Constants: `CHUNK_TIMEOUT_SECS = 8`, `ANKY_THRESHOLD_SECS = 480.0`
- Functions: `start_session`, `send_chunk`, `spawn_session_reaper`, `finalize_non_anky`, `finalize_anky`
- Important difference: active chunk sessions are in-memory only until finalized. If the process dies mid-session, the session is lost.

## 3. API Route Map

Scope note: this list covers Axum `.route(...)` handlers extracted from `src/routes/mod.rs`. Static `nest_service(...)` mounts like `/static`, `/data/images`, and `/agent-skills` are not included below because they are file-serving services, not handler functions.

Counts:
- `.route(...)` call sites in `src/routes/mod.rs`: 232
- Expanded method+path handler edges: 236

Description note: descriptions are condensed from handler doc comments where present; where a route had no doc comment, the description is a minimal summary based on the handler name and file context.

- `POST /api/v1/generate` -> `api::generate_anky_paid` (`src/routes/api.rs`): anky generation Model routing: model="flux" (default) → Flux.1-dev + anky LoRA via ComfyUI, FREE model="gemini" → Gemini image pipeline, PAID ($0.25) Payment (only required for gemini): 1. API key with free agent sessions → free 2. PAYMENT-SIGNATURE / x-payment header → wallet tx hash or x402 3. Nothing → 402 Payment Required.
- `POST /api/v1/prompt` -> `prompt::create_prompt_api` (`src/routes/prompt.rs`): create prompt (with payment).
- `POST /api/v1/prompt/create` -> `prompt::create_prompt_api` (`src/routes/prompt.rs`): create prompt (with payment).
- `POST /api/v1/prompt/quick` -> `prompt::create_prompt_quick` (`src/routes/prompt.rs`): create a shareable prompt link (free, no image) Returns { prompt_id, url } — the prompt can be opened via /write?p={id}.
- `POST /api/v1/studio/upload` -> `api::upload_studio_video` (`src/routes/api.rs`): multipart: video (WebM blob) + metadata (JSON).
- `POST /api/v1/media-factory/video` -> `api::media_factory_video` (`src/routes/api.rs`): submit a Grok video generation request.
- `POST /api/v1/media-factory/image` -> `api::media_factory_image` (`src/routes/api.rs`): generate an image with Gemini.
- `POST /api/v1/media-factory/flux` -> `api::media_factory_flux` (`src/routes/api.rs`): generate an image with Flux via local ComfyUI.
- `POST /api/v1/transform` -> `extension_api::transform` (`src/routes/extension_api.rs`): Handle transform.
- `GET /api/v1/balance` -> `extension_api::balance` (`src/routes/extension_api.rs`): Handle balance.
- `POST /swift/v1/auth/privy` -> `swift::auth_privy` (`src/routes/swift.rs`): Verify a Privy auth token and return a session token for subsequent mobile requests. The returned `session_token` must be stored securely (iOS Keychain) and sent as `Authorization: Bearer <session_token>` on every request.
- `POST /swift/v2/auth/challenge` -> `swift::auth_seed_challenge` (`src/routes/swift.rs`): Create a one-time sign-in challenge for a locally derived Base/EVM seed identity. The iOS app should sign the returned `message` using EIP-191 / personal_sign semantics.
- `POST /swift/v2/auth/verify` -> `swift::auth_seed_verify` (`src/routes/swift.rs`): Verify an EVM signature from the seed-derived keypair and return a normal Anky session token.
- `DELETE /swift/v1/auth/session` -> `swift::auth_logout` (`src/routes/swift.rs`): invalidate the current bearer token.
- `DELETE /swift/v2/auth/session` -> `swift::auth_logout` (`src/routes/swift.rs`): invalidate the current bearer token.
- `GET /swift/v1/me` -> `swift::get_me` (`src/routes/swift.rs`): Get me.
- `GET /swift/v2/me` -> `swift::get_me` (`src/routes/swift.rs`): Get me.
- `GET /swift/v1/writings` -> `swift::list_writings` (`src/routes/swift.rs`): List writings.
- `GET /swift/v2/writings` -> `swift::list_writings` (`src/routes/swift.rs`): List writings.
- `POST /swift/v1/write` -> `swift::submit_writing_unified` (`src/routes/swift.rs`): and /swift/v2/write — unified mobile writing handler. Design: persist the raw data as fast as possible, return immediately with what the frontend needs to start evolving the UI, then spawn all processing in the background. Nothing blocks. The frontend polls /status to watch the downstream artifacts materialize.
- `POST /swift/v2/write` -> `swift::submit_writing_unified` (`src/routes/swift.rs`): and /swift/v2/write — unified mobile writing handler. Design: persist the raw data as fast as possible, return immediately with what the frontend needs to start evolving the UI, then spawn all processing in the background. Nothing blocks. The frontend polls /status to watch the downstream artifacts materialize.
- `GET /swift/v2/writing/{sessionId}/status` -> `swift::get_writing_status` (`src/routes/swift.rs`): Get writing status.
- `GET /swift/v2/children` -> `swift::list_children` (`src/routes/swift.rs`): List children.
- `POST /swift/v2/children` -> `swift::create_child_profile` (`src/routes/swift.rs`): Create child profile.
- `GET /swift/v2/children/{childId}` -> `swift::get_child_profile` (`src/routes/swift.rs`): Get child profile.
- `GET /swift/v2/cuentacuentos/ready` -> `swift::cuentacuentos_ready` (`src/routes/swift.rs`): Handle cuentacuentos ready.
- `GET /swift/v2/cuentacuentos/history` -> `swift::cuentacuentos_history` (`src/routes/swift.rs`): Handle cuentacuentos history.
- `POST /swift/v2/cuentacuentos/{id}/complete` -> `swift::complete_cuentacuentos` (`src/routes/swift.rs`): Complete cuentacuentos.
- `POST /swift/v2/cuentacuentos/{id}/assign` -> `swift::assign_cuentacuentos` (`src/routes/swift.rs`): Assign cuentacuentos.
- `GET /swift/v2/prompt/{id}` -> `swift::get_prompt_by_id` (`src/routes/swift.rs`): fetch a prompt by ID (for deep links, no auth required).
- `GET /swift/v2/next-prompt` -> `swift::get_next_prompt` (`src/routes/swift.rs`): Returns the precomputed writing prompt for this user. If no personalized prompt exists yet, returns a default one.
- `GET /swift/v2/chat/prompt` -> `swift::get_chat_prompt` (`src/routes/swift.rs`): Returns Anky's opening message for a new writing session. This is pre-computed by the post-writing pipeline — no on-demand LLM call. First-ever user: generic. Returning user: reads from next_prompts table.
- `GET /swift/v2/you` -> `swift::get_you` (`src/routes/swift.rs`): Returns what anky knows about you — your profile built from all your writing. Combines local profile data with Honcho's accumulated peer context.
- `GET /swift/v2/you/ankys` -> `swift::get_you_ankys` (`src/routes/swift.rs`): Returns the user's completed ankys for the profile grid.
- `POST /swift/v2/device-token` -> `swift::register_device` (`src/routes/swift.rs`): Register an APNs device token for push notifications. Upserts on (user_id, platform).
- `POST /swift/v2/devices` -> `swift::register_device` (`src/routes/swift.rs`): Register an APNs device token for push notifications. Upserts on (user_id, platform).
- `DELETE /swift/v2/devices` -> `swift::delete_device` (`src/routes/swift.rs`): Remove device token for this user+platform (called on logout).
- `GET /swift/v2/settings` -> `swift::get_settings` (`src/routes/swift.rs`): Get settings.
- `PATCH /swift/v2/settings` -> `swift::patch_settings` (`src/routes/swift.rs`): Update settings.
- `POST /swift/v2/writing/{sessionId}/prepare-mint` -> `swift::prepare_mint` (`src/routes/swift.rs`): Prepare mint.
- `POST /swift/v2/writing/{sessionId}/confirm-mint` -> `swift::confirm_mint` (`src/routes/swift.rs`): Confirm mint.
- `POST /swift/v1/admin/premium` -> `swift::set_premium` (`src/routes/swift.rs`): toggle premium for a user (simple internal endpoint) Body: { "user_id": "...", "is_premium": true }.
- `GET /` -> `pages::home` (`src/routes/pages.rs`): Render the landing page.
- `GET /write` -> `pages::write_page` (`src/routes/pages.rs`): Render the main 8-minute writing interface.
- `GET /stories` -> `pages::stories_page` (`src/routes/pages.rs`): Render the stories page.
- `GET /you` -> `pages::you_page` (`src/routes/pages.rs`): Render the you page.
- `GET /test` -> `pages::test_page` (`src/routes/pages.rs`): Render the test page.
- `GET /gallery` -> `pages::gallery` (`src/routes/pages.rs`): Render the gallery page.
- `GET /gallery/dataset-round-two` -> `pages::dataset_round_two` (`src/routes/pages.rs`): Render the dataset round two page.
- `GET /gallery/dataset-round-two/og-image` -> `pages::dataset_og_image` (`src/routes/pages.rs`): Render the dataset og image page.
- `POST /gallery/dataset-round-two/eliminate` -> `pages::dataset_eliminate` (`src/routes/pages.rs`): Render the dataset eliminate page.
- `GET /video-gallery` -> `pages::videos_gallery` (`src/routes/pages.rs`): Render the videos gallery page.
- `GET /feed` -> `pages::feed_page` (`src/routes/pages.rs`): Render the feed page.
- `GET /help` -> `pages::help` (`src/routes/pages.rs`): Render the help page.
- `GET /mobile` -> `pages::mobile` (`src/routes/pages.rs`): Render the mobile page.
- `GET /dca` -> `pages::dca_dashboard` (`src/routes/pages.rs`): Render the dca dashboard page.
- `GET /dca-bot-code` -> `pages::dca_bot_code` (`src/routes/pages.rs`): Render the dca bot code page.
- `GET /login` -> `pages::login_page` (`src/routes/pages.rs`): Render the login page.
- `GET /ankycoin` -> `pages::ankycoin_page` (`src/routes/pages.rs`): Render the Farcaster-facing Ankycoin miniapp experience.
- `GET /leaderboard` -> `pages::leaderboard` (`src/routes/pages.rs`): Render the leaderboard page.
- `GET /pitch` -> `pages::pitch` (`src/routes/pages.rs`): Render the pitch page.
- `GET /generate` -> `pages::generate_page` (`src/routes/pages.rs`): Render the generate page.
- `GET /create-videos` -> `pages::create_videos_page` (`src/routes/pages.rs`): Render the create videos page.
- `GET /generate/video` -> `pages::video_dashboard` (`src/routes/pages.rs`): Render the video dashboard page.
- `GET /video/pipeline` -> `pages::video_pipeline_page` (`src/routes/pages.rs`): Render the video pipeline page.
- `GET /video-dashboard` -> `pages::media_dashboard` (`src/routes/pages.rs`): Render the media dashboard page.
- `GET /sleeping` -> `pages::sleeping` (`src/routes/pages.rs`): Render the sleeping page.
- `GET /feedback` -> `pages::feedback` (`src/routes/pages.rs`): Render the feedback page.
- `GET /changelog` -> `pages::changelog` (`src/routes/pages.rs`): Render the changelog page.
- `GET /classes` -> `pages::classes_index` (`src/routes/pages.rs`): List all classes.
- `GET /classes/{number}` -> `pages::class_page` (`src/routes/pages.rs`): Render the class page.
- `GET /simulations` -> `simulations::simulations_page` (`src/routes/simulations.rs`): Render or serve simulations page.
- `GET /api/simulations/slots` -> `simulations::slots_status` (`src/routes/simulations.rs`): Handle slots status.
- `GET /api/simulations/slots/stream` -> `simulations::slots_stream` (`src/routes/simulations.rs`): Stream slots updates.
- `POST /api/simulations/slots/demo` -> `simulations::slots_demo` (`src/routes/simulations.rs`): Handle slots demo.
- `GET /llm` -> `pages::llm` (`src/routes/pages.rs`): Render the llm page.
- `GET /pitch-deck` -> `pages::pitch_deck` (`src/routes/pages.rs`): Render the pitch deck page.
- `GET /pitch-deck.pdf` -> `pages::pitch_deck_pdf` (`src/routes/pages.rs`): Serve the auto-generated pitch deck PDF.
- `POST /api/v1/llm/training-status` -> `api::llm_training_status` (`src/routes/api.rs`): Handle llm training status.
- `POST /api/v1/classes/generate` -> `api::generate_class` (`src/routes/api.rs`): Body: { "title": "...", "concept": "...", "slides": [{"heading":"...","body":"...","code":"...","file":"...","note":"..."}, ...] } Stores a programming class with 8 text+code slides.
- `GET /anky/{id}` -> `pages::anky_detail` (`src/routes/pages.rs`): Render the anky detail page.
- `GET /story/{story_id}` -> `voices::story_deep_link_page` (`src/routes/voices.rs`): Render or serve story deep link page.
- `GET /api/og/write` -> `api::og_write_svg` (`src/routes/api.rs`): ?prompt=... — dynamic SVG OG image for prompt share links.
- `GET /prompt` -> `prompt::prompt_new_page` (`src/routes/prompt.rs`): simple prompt creation page (free, generates shareable link).
- `GET /prompt/create` -> `prompt::create_prompt_page` (`src/routes/prompt.rs`): form to write a prompt + pay.
- `GET /prompt/{id}` -> `prompt::prompt_page` (`src/routes/prompt.rs`): writing page for prompt (with OG tags).
- `GET /api/v1/prompt/{id}` -> `prompt::get_prompt_api` (`src/routes/prompt.rs`): poll prompt status/details.
- `POST /api/v1/prompt/{id}/write` -> `prompt::submit_prompt_writing` (`src/routes/prompt.rs`): submit writing session for a prompt.
- `GET /api/v1/prompts` -> `prompt::list_prompts_api` (`src/routes/prompt.rs`): paginated list of completed prompts.
- `GET /api/v1/prompts/random` -> `prompt::random_prompt_api` (`src/routes/prompt.rs`): random completed prompt.
- `GET /settings` -> `settings::settings_page` (`src/routes/settings.rs`): settings page (requires auth).
- `POST /api/settings` -> `settings::save_settings` (`src/routes/settings.rs`): save user settings.
- `POST /api/claim-username` -> `settings::claim_username` (`src/routes/settings.rs`): lightweight endpoint to just claim a username.
- `GET /auth/x/login` -> `auth::login` (`src/routes/auth.rs`): initiate X OAuth 2.0 PKCE flow.
- `GET /auth/x/callback` -> `auth::callback` (`src/routes/auth.rs`): handle OAuth callback.
- `GET /auth/x/logout` -> `auth::logout` (`src/routes/auth.rs`): session and cookies.
- `POST /auth/privy/verify` -> `auth::privy_verify` (`src/routes/auth.rs`): Handle privy verify.
- `POST /auth/privy/logout` -> `auth::privy_logout` (`src/routes/auth.rs`): clear Privy session cookies.
- `POST /auth/seed/verify` -> `auth::seed_verify` (`src/routes/auth.rs`): Web-side seed identity verification. Same logic as /swift/v2/auth/verify but sets browser cookies.
- `POST /auth/seed/logout` -> `auth::seed_logout` (`src/routes/auth.rs`): clear seed session cookies and invalidate session.
- `POST /auth/farcaster/verify` -> `auth::farcaster_verify` (`src/routes/auth.rs`): Handle farcaster verify.
- `POST /write` -> `writing::process_writing` (`src/routes/writing.rs`): Accept a completed browser writing session and trigger post-write processing.
- `GET /writings` -> `writing::get_writings` (`src/routes/writing.rs`): Get writings.
- `GET /api/writing/{sessionId}/status` -> `writing::get_writing_status_web` (`src/routes/writing.rs`): web-accessible writing status (cookie auth).
- `POST /collection/create` -> `collection::create_collection` (`src/routes/collection.rs`): Create collection.
- `GET /collection/{id}` -> `collection::get_collection` (`src/routes/collection.rs`): Get collection.
- `POST /payment/verify` -> `payment::verify_payment` (`src/routes/payment.rs`): Verify payment.
- `POST /notify/signup` -> `notification::signup` (`src/routes/notification.rs`): Handle signup.
- `GET /api/ankys` -> `api::list_ankys` (`src/routes/api.rs`): List ankys.
- `GET /api/v1/ankys` -> `api::list_ankys` (`src/routes/api.rs`): List ankys.
- `POST /api/generate` -> `api::generate_anky` (`src/routes/api.rs`): Handle generate anky.
- `GET /api/v1/anky/{id}` -> `api::get_anky` (`src/routes/api.rs`): fetch anky details (for polling after /write) Writing text is only included if the requester's anky_user_id cookie matches the anky's owner.
- `GET /api/v1/mind/status` -> `api::get_mind_status` (`src/routes/api.rs`): check Mind (llama-server) availability and slot status.
- `GET /api/v1/anky/{id}/metadata` -> `swift::anky_metadata` (`src/routes/swift.rs`): public ERC1155-compliant metadata.
- `GET /api/stream-reflection/{id}` -> `api::stream_reflection` (`src/routes/api.rs`): stream title+reflection from Claude via SSE. If reflection already exists in DB, sends it immediately. Otherwise, streams from Claude and saves to DB in the background. CRITICAL: The SSE stream is returned IMMEDIATELY so the browser gets headers right away. The DB lookup and Claude call happen inside the stream's spawned task, not before the response is sent. This prevents DB lock contention from blocking the SSE connection establishment.
- `POST /api/warm-context` -> `api::warm_context` (`src/routes/api.rs`): pre-build Honcho + memory context while user is still writing. Called by frontend at minute 6 so context is ready when reflection starts.
- `GET /api/me` -> `api::web_me` (`src/routes/api.rs`): web profile using cookie auth.
- `GET /api/my-ankys` -> `api::web_my_ankys` (`src/routes/api.rs`): user's ankys using cookie auth.
- `GET /api/chat-history` -> `api::web_chat_history` (`src/routes/api.rs`): returns the user's session history as a chat timeline. Each session has: user writing (truncated), anky response/reflection, follow-up messages, timestamp.
- `GET /api/anky-card/{id}` -> `api::anky_reflection_card_image` (`src/routes/api.rs`): render a phone-sized downloadable reflection card image.
- `POST /api/checkpoint` -> `api::save_checkpoint` (`src/routes/api.rs`): Persist an in-progress browser writing checkpoint.
- `GET /api/session/paused` -> `api::get_paused_writing_session` (`src/routes/api.rs`): Get paused writing session.
- `POST /api/session/pause` -> `api::pause_writing_session` (`src/routes/api.rs`): Handle pause writing session.
- `POST /api/session/resume` -> `api::resume_writing_session` (`src/routes/api.rs`): Handle resume writing session.
- `POST /api/session/discard` -> `api::discard_paused_writing_session` (`src/routes/api.rs`): Handle discard paused writing session.
- `POST /api/prefetch-memory` -> `api::prefetch_memory` (`src/routes/api.rs`): pre-warm memory context during a writing session. Called at ~5 minutes so the context is ready when the reflection is requested.
- `GET /api/cost-estimate` -> `api::cost_estimate` (`src/routes/api.rs`): Handle cost estimate.
- `GET /api/treasury` -> `api::treasury_address` (`src/routes/api.rs`): Handle treasury address.
- `GET /api/mirror` -> `api::mirror` (`src/routes/api.rs`): ?fid=<u64> Fetches a Farcaster user's profile + recent casts, generates a "public mirror" portrait via Claude, and produces a unique Anky image via ComfyUI.
- `GET /api/mirror/gallery` -> `api::mirror_gallery` (`src/routes/api.rs`): ?limit=50&offset=0 Returns all generated mirrors (without full b64 image — uses image_url instead).
- `POST /api/mirror/chat` -> `api::mirror_chat` (`src/routes/api.rs`): Chat with a mirror's anky — the anky speaks from the mirror context.
- `POST /api/mirror/mint-sig` -> `api::mirror_mint_sig` (`src/routes/api.rs`): EIP-712 signature to mint a mirror NFT. Body: { "mirror_id": "uuid", "minter": "0x..." }.
- `GET /api/mirror/metadata/{id}` -> `api::mirror_metadata` (`src/routes/api.rs`): ERC-721 metadata JSON for a minted mirror.
- `GET /image.png` -> `api::mirror_latest_image` (`src/routes/api.rs`): serves the latest mirror image with PFP overlay composited. Used as the Farcaster frame image for ankycoin.com.
- `GET /splash.png` -> `api::mirror_latest_image` (`src/routes/api.rs`): serves the latest mirror image with PFP overlay composited. Used as the Farcaster frame image for ankycoin.com.
- `POST /api/feedback` -> `api::submit_feedback` (`src/routes/api.rs`): Submit feedback.
- `POST /api/v1/feedback` -> `api::submit_feedback` (`src/routes/api.rs`): Submit feedback.
- `POST /api/chat` -> `api::chat_with_anky` (`src/routes/api.rs`): Handle chat with anky.
- `POST /api/chat-quick` -> `api::chat_quick` (`src/routes/api.rs`): Handle chat quick.
- `POST /api/suggest-replies` -> `api::suggest_replies` (`src/routes/api.rs`): Handle suggest replies.
- `POST /api/retry-failed` -> `api::retry_failed` (`src/routes/api.rs`): Handle retry failed.
- `POST /api/v1/generate/video-frame` -> `api::generate_video_frame` (`src/routes/api.rs`): generate a single video frame image (paid via x402).
- `POST /api/v1/generate/video` -> `api::generate_video` (`src/routes/api.rs`): generate an 88-second video from an anky's writing session. Returns immediately after saving the project to DB. Script generation and video rendering happen entirely in a background task so the browser never times out. The frontend polls GET /api/v1/video/{id} for progress.
- `GET /api/v1/create-videos/{id}` -> `api::get_create_video_card` (`src/routes/api.rs`): fetch prompt state for the marketing video creator.
- `POST /api/v1/create-videos/image` -> `api::generate_create_video_image` (`src/routes/api.rs`): generate the 16:9 seed image for a marketing concept.
- `POST /api/v1/create-videos/video` -> `api::generate_create_video_clip` (`src/routes/api.rs`): animate a generated seed image into a 16:9 Grok clip.
- `GET /api/v1/video/{id}` -> `api::get_video_project` (`src/routes/api.rs`): poll video project status.
- `POST /api/v1/video/{id}/resume` -> `api::resume_video_project` (`src/routes/api.rs`): resume a failed video project from where it left off.
- `GET /api/v1/video/pipeline/config` -> `api::get_video_pipeline_config` (`src/routes/api.rs`): current prompt templates + spend summary.
- `POST /api/v1/video/pipeline/config` -> `api::save_video_pipeline_config` (`src/routes/api.rs`): update prompt templates used by the 8m→88s pipeline.
- `POST /api/v1/purge-cache` -> `api::purge_cache` (`src/routes/api.rs`): purge Cloudflare cache (admin only).
- `GET /og/video` -> `api::og_video_image` (`src/routes/api.rs`): dynamically generate an OG image for the video page.
- `GET /og/dca` -> `api::og_dca_image` (`src/routes/api.rs`): dynamically generate OG image for DCA page with latest buys.
- `GET /api/v1/feed` -> `api::get_feed` (`src/routes/api.rs`): ?page=1&per_page=20.
- `POST /api/v1/anky/{id}/like` -> `api::toggle_like` (`src/routes/api.rs`): toggle like.
- `POST /api/v1/story/test` -> `api::story_test` (`src/routes/api.rs`): test story generation with any model/provider. Requires Bearer auth. Does NOT save to database.
- `GET /admin/story-tester` -> `api::admin_story_tester` (`src/routes/api.rs`): serve the story pipeline tester UI (requires auth).
- `GET /flux-lab` -> `api::flux_lab_page` (`src/routes/api.rs`): serve the flux lab page.
- `GET /api/v1/flux-lab/experiments` -> `api::flux_lab_list_experiments` (`src/routes/api.rs`): list all experiments.
- `GET /api/v1/flux-lab/experiments/{name}` -> `api::flux_lab_get_experiment` (`src/routes/api.rs`): and prompts for an experiment.
- `POST /api/v1/flux-lab/generate` -> `api::flux_lab_generate` (`src/routes/api.rs`): generate images from an array of prompts.
- `POST /api/v1/ankycoin/generate` -> `api::ankycoin_generate_image` (`src/routes/api.rs`): Handle ankycoin generate image.
- `GET /api/v1/ankycoin/latest` -> `api::ankycoin_latest_image` (`src/routes/api.rs`): return the most recently generated ankycoin image + prompt.
- `GET /media-factory` -> `api::media_factory_page` (`src/routes/api.rs`): serve the media factory page.
- `GET /api/v1/media-factory/list` -> `api::media_factory_list` (`src/routes/api.rs`): list all previously generated media factory files.
- `GET /api/v1/media-factory/video/{request_id}` -> `api::media_factory_video_poll` (`src/routes/api.rs`): poll video generation status.
- `POST /api/v1/check-prompt` -> `api::check_prompt` (`src/routes/api.rs`): classify a prompt before payment.
- `GET /api/v1/og-embed` -> `api::og_embed_image` (`src/routes/api.rs`): serves the latest anky's image with title + username overlay as the Farcaster frame embed. Cloudflare caches via Cache-Control.
- `GET /api/v1/stories` -> `swift::list_all_stories` (`src/routes/swift.rs`): public feed of all stories, newest first. No auth required. Returns stories with images decorated.
- `GET /api/v1/stories/{id}` -> `swift::get_story` (`src/routes/swift.rs`): public single story with images.
- `GET /api/v1/stories/{story_id}/recordings` -> `voices::list_recordings` (`src/routes/voices.rs`): List recordings.
- `POST /api/v1/stories/{story_id}/recordings` -> `voices::create_recording` (`src/routes/voices.rs`): Create recording.
- `GET /api/v1/stories/{story_id}/voice` -> `voices::get_voice` (`src/routes/voices.rs`): Get voice.
- `POST /api/v1/stories/{story_id}/recordings/{recording_id}/complete` -> `voices::complete_listen` (`src/routes/voices.rs`): Complete listen.
- `POST /api/v1/register` -> `extension_api::register` (`src/routes/extension_api.rs`): create a new agent with an API key (everything is free).
- `POST /api/v1/session/start` -> `session::start_session` (`src/routes/session.rs`): open a new chunked writing session.
- `POST /api/v1/session/chunk` -> `session::send_chunk` (`src/routes/session.rs`): append text to an active session.
- `GET /api/v1/session/{id}/events` -> `session::session_events` (`src/routes/session.rs`): replay the server-observed session timeline. Requires the same X-API-Key used to create the session.
- `GET /api/v1/session/{id}/result` -> `session::session_result` (`src/routes/session.rs`): recover the final outcome for a session. Requires the same X-API-Key used to create the session.
- `GET /api/v1/session/{id}` -> `session::session_status` (`src/routes/session.rs`): check session status.
- `GET /manifesto.md` -> `manifesto_md` (`src/routes/mod.rs`): Serve the repo manifesto as markdown.
- `GET /MANIFESTO.md` -> `manifesto_md` (`src/routes/mod.rs`): Serve the repo manifesto as markdown.
- `GET /PROMPT.md` -> `prompt_md` (`src/routes/mod.rs`): Serve the repo prompt document as markdown.
- `GET /SOUL.md` -> `soul_md` (`src/routes/mod.rs`): Serve the repo soul document as markdown.
- `GET /prompts/{id}` -> `serve_prompt` (`src/routes/mod.rs`): serve markdown prompt files from prompts/ directory. id must be exactly 4 digits (e.g. 0001).
- `GET /skills` -> `skills` (`src/routes/mod.rs`): Serve the plain-text skills document.
- `GET /skill.md` -> `skill_md` (`src/routes/mod.rs`): Serve the installable Anky skill file.
- `GET /skill` -> `skills_redirect` (`src/routes/mod.rs`): Redirect legacy skill URLs to `/skills`.
- `GET /skills.md` -> `skills_redirect` (`src/routes/mod.rs`): Redirect legacy skill URLs to `/skills`.
- `GET /agent-skills/anky` -> `anky_skill_bundle` (`src/routes/mod.rs`): Serve the installable skill bundle descriptor.
- `GET /agent-skills/anky/` -> `anky_skill_bundle` (`src/routes/mod.rs`): Serve the installable skill bundle descriptor.
- `GET /agent-skills/anky/skill.md` -> `anky_skill_bundle_entry_redirect` (`src/routes/mod.rs`): Redirect bundle aliases to the skill entry file.
- `GET /agent-skills/anky/skills.md` -> `anky_skill_bundle_entry_redirect` (`src/routes/mod.rs`): Redirect bundle aliases to the skill entry file.
- `GET /agent-skills/anky/manifest.json` -> `anky_skill_bundle_manifest` (`src/routes/mod.rs`): Serve the skill bundle manifest JSON.
- `GET /api/ankys/today` -> `live::todays_ankys` (`src/routes/live.rs`): JSON list of today's completed ankys with images.
- `GET /api/live-status` -> `live::live_status_sse` (`src/routes/live.rs`): SSE stream of live status changes.
- `GET /interview` -> `interview::interview_page` (`src/routes/interview.rs`): Render or serve interview page.
- `GET /ws/interview` -> `interview::ws_interview_proxy` (`src/routes/interview.rs`): WebSocket proxy to Python interview engine on port 8890.
- `POST /api/interview/start` -> `interview::interview_start` (`src/routes/interview.rs`): Handle interview start.
- `POST /api/interview/message` -> `interview::interview_message` (`src/routes/interview.rs`): Handle interview message.
- `POST /api/interview/end` -> `interview::interview_end` (`src/routes/interview.rs`): Handle interview end.
- `GET /api/interview/history/{user_id}` -> `interview::interview_history` (`src/routes/interview.rs`): Handle interview history.
- `GET /api/interview/user-context/{user_id}` -> `interview::interview_user_context` (`src/routes/interview.rs`): context/:user_id.
- `GET /stream/overlay` -> `pages::stream_overlay` (`src/routes/pages.rs`): Render the stream overlay page.
- `GET /generations` -> `generations::list_batches` (`src/routes/generations.rs`): list all batches.
- `GET /generations/{id}` -> `generations::review_batch` (`src/routes/generations.rs`): review a prompt batch.
- `POST /generations/{id}/status` -> `generations::save_status` (`src/routes/generations.rs`): save keep/skip decisions.
- `GET /generations/{id}/dashboard` -> `generations::generation_dashboard` (`src/routes/generations.rs`): live generation dashboard.
- `GET /generations/{id}/progress` -> `generations::generation_progress` (`src/routes/generations.rs`): returns progress.json for the batch.
- `GET /generations/{id}/tinder` -> `generations::review_images` (`src/routes/generations.rs`): keyboard-driven approve/reject review of generated images.
- `POST /generations/{id}/review` -> `generations::save_review` (`src/routes/generations.rs`): save approve/reject for an image.
- `GET /training` -> `training::training_page` (`src/routes/training.rs`): Render or serve training page.
- `GET /trainings` -> `training::trainings_list` (`src/routes/training.rs`): Render or serve trainings list.
- `GET /trainings/general-instructions` -> `training::general_instructions` (`src/routes/training.rs`): Render or serve general instructions.
- `GET /trainings/{date}` -> `training::training_run_detail` (`src/routes/training.rs`): Render or serve training run detail.
- `GET /api/training/next` -> `training::next_image` (`src/routes/training.rs`): Handle next image.
- `POST /api/training/vote` -> `training::vote` (`src/routes/training.rs`): Handle vote.
- `POST /api/training/heartbeat` -> `training::training_heartbeat` (`src/routes/training.rs`): RunPod watcher pushes state here.
- `GET /api/training/state` -> `training::training_state` (`src/routes/training.rs`): returns current training state + sample image list.
- `GET /training/live` -> `training::training_live` (`src/routes/training.rs`): live training dashboard page.
- `GET /training/live/samples/{filename}` -> `training::training_sample_image` (`src/routes/training.rs`): serve sample images.
- `POST /api/memory/backfill` -> `api::memory_backfill` (`src/routes/api.rs`): backfill memory for all existing writing sessions.
- `GET /evolve` -> `evolve::evolve_dashboard` (`src/routes/evolve.rs`): Render or serve evolve dashboard.
- `GET /dashboard` -> `dashboard::dashboard` (`src/routes/dashboard.rs`): Render or serve dashboard.
- `GET /dashboard/logs` -> `dashboard::dashboard_logs` (`src/routes/dashboard.rs`): Render or serve dashboard logs.
- `GET /dashboard/summaries` -> `dashboard::dashboard_summaries` (`src/routes/dashboard.rs`): return recent system summaries as JSON.
- `GET /.well-known/apple-app-site-association` -> `apple_app_site_association` (`src/routes/mod.rs`): Serve the Apple Universal Links association JSON.
- `GET /.well-known/farcaster.json` -> `farcaster_manifest` (`src/routes/mod.rs`): Serve the Farcaster miniapp manifest JSON.
- `GET /.well-known/agent` -> `agent_manifest` (`src/routes/mod.rs`): Serve the agent registry manifest.
- `GET /sw.js` -> `service_worker` (`src/routes/mod.rs`): Serve the root-scoped service worker script.
- `GET /webhooks/x` -> `webhook_x::webhook_crc` (`src/routes/webhook_x.rs`): Handle crc.
- `POST /webhooks/x` -> `webhook_x::webhook_post` (`src/routes/webhook_x.rs`): Handle post.
- `POST /webhooks/farcaster` -> `webhook_farcaster::webhook_post` (`src/routes/webhook_farcaster.rs`): Handle post.
- `GET /webhooks/logs` -> `webhook_x::webhook_logs_page` (`src/routes/webhook_x.rs`): Handle logs page.
- `GET /webhooks/logs/stream` -> `webhook_x::webhook_logs_stream` (`src/routes/webhook_x.rs`): Stream webhook logs updates.
- `GET /health` -> `health::health_check` (`src/routes/health.rs`): Render or serve health check.

## 4. Infrastructure & Services

### 4.1 systemd units found

Repo-tracked unit files:
- `deploy/anky-mind.service`
- `deploy/anky-heart.service`

Installed workstation units inspected under `/etc/systemd/system`:
- `anky.service`
- `anky-mind.service`
- `anky-heart.service`
- `ollama.service`
- `poiesis-web.service`
- `valkey.service` is installed from `/usr/lib/systemd/system/valkey.service`

Enabled/disabled state:

```text
anky-heart.service                                                            enabled         disabled
anky-mind.service                                                             enabled         disabled
anky.service                                                                  enabled         disabled
ollama.service                                                                disabled        disabled
poiesis-web.service                                                           enabled         disabled
valkey-sentinel.service                                                       disabled        disabled
valkey.service                                                                enabled         disabled
```

### 4.2 Main app service

`/etc/systemd/system/anky.service`:

```ini
[Unit]
Description=Anky Server
After=network.target valkey.service

[Service]
Type=simple
User=kithkui
WorkingDirectory=/home/kithkui/anky
EnvironmentFile=/home/kithkui/anky/.env
ExecStart=/home/kithkui/anky/target/release/anky
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

Implications:
- The Rust server is launched directly from `target/release/anky`.
- Runtime configuration comes from `/home/kithkui/anky/.env`.
- The service explicitly starts after `valkey.service`.

### 4.3 llama-server / Mind

Repo template `deploy/anky-mind.service`:

```ini
[Unit]
Description=Anky Mind — llama-server qwen3.5-27b (GPU 0)
After=network.target

[Service]
Type=simple
User=kithkui
Environment=CUDA_VISIBLE_DEVICES=0
ExecStart=/usr/local/bin/llama-server \
    --model /home/kithkui/models/qwen3.5-27b-q4_k_m.gguf \
    --host 127.0.0.1 \
    --port 8080 \
    --n-gpu-layers 99 \
    --parallel 8 \
    --ctx-size 2048 \
    --batch-size 512 \
    --ubatch-size 512 \
    --cont-batching \
    --no-mmap \
    --log-disable
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

Installed `/etc/systemd/system/anky-mind.service`:

```ini
[Unit]
Description=Anky Mind — llama-server qwen3.5-27b (GPU 0)
After=network.target

[Service]
Type=simple
User=kithkui
Environment=CUDA_VISIBLE_DEVICES=0
ExecStart=/usr/local/bin/llama-server \
    --model /home/kithkui/models/qwen3.5-27b-q4_k_m.gguf \
    --host 127.0.0.1 \
    --port 8080 \
    --n-gpu-layers 99 \
    --parallel 2 \
    --ctx-size 32768 \
    --batch-size 512 \
    --ubatch-size 512 \
    --cont-batching \
    --no-mmap \
    --log-disable
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

Observed runtime configuration:
- Binary: `/usr/local/bin/llama-server`
- Model: `/home/kithkui/models/qwen3.5-27b-q4_k_m.gguf`
- Host/port: `127.0.0.1:8080`
- GPU: `CUDA_VISIBLE_DEVICES=0`
- Parallelism: installed unit uses `--parallel 2`
- Context size: installed unit uses `--ctx-size 32768`
- API shape: OpenAI-compatible `/v1/chat/completions`, consumed by `src/services/mind.rs`

Important drift:
- The repo template still says `--parallel 8` and `--ctx-size 2048`.
- The installed unit is materially different from the checked-in unit.

### 4.4 ComfyUI / image generation

Repo template `deploy/anky-heart.service`:

```ini
[Unit]
Description=Anky Heart — ComfyUI Flux LoRA (GPU 1)
After=network.target

[Service]
Type=simple
User=kithkui
Environment=CUDA_VISIBLE_DEVICES=1
WorkingDirectory=/home/kithkui/ComfyUI
ExecStart=/home/kithkui/ComfyUI/venv/bin/python main.py \
    --listen 127.0.0.1 \
    --port 8188 \
    --disable-auto-launch
Restart=always
RestartSec=15
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

Installed `/etc/systemd/system/anky-heart.service`:

```ini
[Unit]
Description=Anky Heart — ComfyUI Flux LoRA (GPU 1)
After=network.target

[Service]
Type=simple
User=kithkui
Environment=CUDA_VISIBLE_DEVICES=1
WorkingDirectory=/home/kithkui/ComfyUI
ExecStart=/home/kithkui/ComfyUI/venv/bin/python main.py \
    --listen 127.0.0.1 \
    --port 8188 \
    --disable-auto-launch
Restart=always
RestartSec=15
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

Integration points:
- `src/services/comfyui.rs`: `generate_image`, `generate_image_sized`, `generate_story_image`
- `src/pipeline/image_gen.rs`: `generate_anky_from_writing` falls back to ComfyUI if Gemini image generation fails
- Story/cuentacuentos image generation also uses the ComfyUI workflow path

Important config issue:
- `Config.comfyui_url` exists in `src/config.rs`, but the primary ComfyUI client still hardcodes `127.0.0.1:8188`. The env var is not the single source of truth.

### 4.5 Valkey / Redis

Service definition:

```ini
# /usr/lib/systemd/system/valkey.service
[Unit]
Description=Valkey persistent key-value database
After=network.target
After=network-online.target
Wants=network-online.target

[Service]
WorkingDirectory=/var/lib/valkey
# ensure var is set
Environment=OPTIONS=
EnvironmentFile=-/etc/sysconfig/valkey
# we must keep $OPTIONS and the env file as some older installs will still be using /etc/sysconfig/valkey
ExecStart=/usr/bin/valkey-server /etc/valkey/valkey.conf --daemonize no --supervised systemd $OPTIONS
Type=notify
User=valkey
Group=valkey
RuntimeDirectory=valkey
RuntimeDirectoryMode=0755

# If you need to change max open file limit
# for example, when you change maxclient in configuration
# you can change the LimitNOFILE value below.
# See "man systemd.exec" for more information.
LimitNOFILE=10240

# Slave nodes on large system may take lot of time to start.
# You may need to uncomment TimeoutStartSec and TimeoutStopSec
# directives below and raise their value.
# See "man systemd.service" for more information.
#TimeoutStartSec=90s
#TimeoutStopSec=90s

[Install]
WantedBy=multi-user.target


# /usr/lib/systemd/system/service.d/10-timeout-abort.conf
# This file is part of the systemd package.
# See https://fedoraproject.org/wiki/Changes/Shorter_Shutdown_Timer.
#
# To facilitate debugging when a service fails to stop cleanly,
# TimeoutStopFailureMode=abort is set to "crash" services that fail to stop in
# the time allotted. This will cause the service to be terminated with SIGABRT
# and a coredump to be generated.
#
# To undo this configuration change, create a mask file:
#   sudo mkdir -p /etc/systemd/system/service.d
#   sudo ln -sv /dev/null /etc/systemd/system/service.d/10-timeout-abort.conf

[Service]
TimeoutStopFailureMode=abort
```

Observed server info:

```text
# Server
redis_version:7.2.4
server_name:valkey
valkey_version:8.1.6
valkey_release_stage:ga
redis_git_sha1:00000000
redis_git_dirty:0
redis_build_id:e15910f780b17c6a
server_mode:standalone
os:Linux 6.17.12-200.nobara.fc43.x86_64 x86_64
arch_bits:64
monotonic_clock:POSIX clock_gettime
multiplexing_api:epoll
gcc_version:15.2.1
process_id:2864
process_supervised:systemd
run_id:8d5bc6a3cb3b53da3d38847b99cc21a4cd593cce
tcp_port:6379
server_time_usec:1774888477487930
uptime_in_seconds:99490
uptime_in_days:1
hz:10
configured_hz:10
clients_hz:10
lru_clock:13280797
executable:/usr/bin/valkey-server
config_file:/etc/valkey/valkey.conf
io_threads_active:0
availability_zone:
listener0:name=tcp,bind=127.0.0.1,bind=-::1,port=6379
listener1:name=unix,bind=/run/valkey/valkey.sock
```

Queue code in repo:
- `src/services/redis_queue.rs`
  - `PRO_QUEUE = anky:jobs:pro`
  - `FREE_QUEUE = anky:jobs:free`
  - processing keys: `anky:jobs:processing:{job_id}`
- Startup hook: `src/main.rs` calls `recover_processing_jobs(&state.config.redis_url)`.

Reality check:
- The live GPU pipeline does **not** currently pop/push work through Redis. It uses `state::GpuJobQueue`, an in-memory dual-channel queue in `src/state.rs`.
- `src/services/mod.rs` marks `redis_queue` with `#[allow(dead_code)]`.
- Current matching keys from the local Valkey instance: `(none at audit time)`

### 4.6 Other local services / external integrations

Local services and bridges found in source:
- Interview engine: `interview-engine/server.py` on port `8890`; browser traffic reaches it through `src/routes/interview.rs` -> `ws_interview_proxy`.
- Hermes bridge: `src/services/hermes.rs` hardcodes `http://127.0.0.1:8891`.
- TTS: `src/services/tts.rs` expects `TTS_BASE_URL` (default `http://localhost:5001`). No matching local systemd unit was found in `/etc/systemd/system` during this audit.
- Whisper fallback: `src/routes/voices.rs` posts to `http://localhost:8080/inference`, which currently collides with the installed Mind/llama-server port.

External services used by the codebase:
- Anthropic Claude
- Gemini
- OpenRouter
- Honcho
- Neynar/Farcaster
- Cloudflare R2
- Pinata
- Base RPC
- X/Twitter APIs

### 4.7 Port allocations

| Port | Service | Source |
| --- | --- | --- |
| `8889` | Main Axum server (default) | `src/config.rs`, `.env`, `/etc/systemd/system/anky.service` |
| `8080` | Mind / llama-server | `/etc/systemd/system/anky-mind.service`, `src/services/mind.rs` |
| `8188` | ComfyUI | `/etc/systemd/system/anky-heart.service`, `src/services/comfyui.rs` |
| `6379` | Valkey | `redis-cli INFO server`, `valkey.service` |
| `11434` | Ollama default API | `src/config.rs` default; `ollama.service` exists but is disabled |
| `8890` | Interview engine websocket server | `interview-engine/server.py`, `src/routes/interview.rs` |
| `8891` | Hermes bridge | `src/services/hermes.rs` |
| `5001` | F5-TTS service (expected) | `src/config.rs`, `src/services/tts.rs` |
| `3030` | `poiesis-web` | `/etc/systemd/system/poiesis-web.service` |
| `8000` | Local/dev Honcho base URL seen in env usage | config/examples and local workstation setup |

### 4.8 Poiesis sidecar service

`/etc/systemd/system/poiesis-web.service`:

```ini
[Unit]
Description=Poiesis Web — jpfraneto.com
After=network.target
Wants=ollama.service

[Service]
Type=simple
User=kithkui
WorkingDirectory=/home/kithkui/poiesis-web
ExecStart=/home/kithkui/poiesis-web/target/release/poiesis-web
Restart=on-failure
RestartSec=5s
Environment=RUST_LOG=info
Environment=PORT=3030
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

This is adjacent to the Anky monorepo, not part of the main Axum app, but it shares the workstation and consumes Ollama as a dependency.

## 5. Data Flow Diagram (text-based)

```text
Browser user
  |
  | GET /write
  v
Axum page handler (`pages::write_page`)
  |
  | renders `templates/home.html`
  v
Browser JS session loop
  |-- keydown/input events
  |-- 8-second inactivity enforced in `tick()`
  |-- periodic POST /api/checkpoint
  |-- POST /api/warm-context near minute 6
  v
POST /write (`writing::process_writing`)
  |
  |-- persist `writing_sessions` row in SQLite
  |-- if short/non-anky: immediate Claude Haiku reply
  |     |-- store response / later store mood + next prompt
  |     '-- browser polls GET /api/writing/{sessionId}/status
  |
  '-- if full anky:
        |-- insert pending `ankys` row in SQLite
        |-- enqueue `GpuJob::AnkyImage` into in-memory GPU queue
        v
      `gpu_job_worker` (`src/main.rs`)
        |
        '-- `image_gen::generate_anky_from_writing`
              |-- local Mind / Claude / Gemini build image prompt
              |-- Gemini image generation first
              |-- fallback to ComfyUI on 127.0.0.1:8188
              |-- write PNG/WebP/thumb files
              |-- optional upload to Cloudflare R2
              |-- save `.anky` story string to DB
              '-- mark `ankys` row complete

Parallel reflection path for ankys:
Browser EventSource /api/stream-reflection/{ankyId}
  v
`api::stream_reflection`
  |-- if reflection already saved, stream cached text
  |-- else stream Claude response live
  |-- fallback to Haiku/OpenRouter if needed
  '-- save title + reflection into SQLite

Final user-visible outputs:
- Immediate JSON response from POST /write
- Polling response from GET /api/writing/{sessionId}/status
- SSE reflection stream from GET /api/stream-reflection/{ankyId}
- Canonical detail page GET /anky/{id}
```

## 6. Database / State

### 6.1 Primary database

Primary live state store: SQLite at `data/anky.db`

DB access model:
- Opened in `src/main.rs` via `db::open_db("data/anky.db")`
- Wrapped in `AppState.db: Arc<Mutex<rusqlite::Connection>>` in `src/state.rs`
- WAL enabled in `src/db/mod.rs`

Schema excerpts inspected directly from SQLite:

`writing_sessions`:

```sql
CREATE TABLE writing_sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            content TEXT NOT NULL,
            duration_seconds REAL NOT NULL,
            word_count INTEGER NOT NULL,
            is_anky BOOLEAN NOT NULL DEFAULT 0,
            response TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')), keystroke_deltas TEXT, flow_score REAL, status TEXT NOT NULL DEFAULT 'completed', pause_used BOOLEAN NOT NULL DEFAULT 0, paused_at TEXT, resumed_at TEXT, session_token TEXT, content_deleted_at TEXT, anky_response TEXT, anky_next_prompt TEXT, anky_mood TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id)
        );
```

`ankys`:

```sql
CREATE TABLE ankys (
            id TEXT PRIMARY KEY,
            writing_session_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            image_prompt TEXT,
            reflection TEXT,
            title TEXT,
            image_path TEXT,
            caption TEXT,
            thinker_name TEXT,
            thinker_moment TEXT,
            is_minted BOOLEAN NOT NULL DEFAULT 0,
            mint_tx_hash TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL DEFAULT (datetime('now')), origin TEXT NOT NULL DEFAULT 'written', image_webp TEXT, image_thumb TEXT, conversation_json TEXT, image_model TEXT NOT NULL DEFAULT 'gemini', prompt_id TEXT, formatted_writing TEXT, gas_funded_at TEXT, session_cid TEXT, metadata_uri TEXT, token_id TEXT, anky_story TEXT, kingdom_id INTEGER, kingdom_name TEXT, kingdom_chakra TEXT, retry_count INTEGER NOT NULL DEFAULT 0, last_retry_at TEXT,
            FOREIGN KEY (writing_session_id) REFERENCES writing_sessions(id),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );
```

`users`:

```sql
CREATE TABLE users (
            id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        , username TEXT, wallet_address TEXT, privy_did TEXT, farcaster_fid INTEGER, farcaster_username TEXT, farcaster_pfp_url TEXT, email TEXT, is_premium BOOLEAN NOT NULL DEFAULT 0, premium_since TEXT, generated_wallet_secret TEXT, wallet_generated_at TEXT, is_pro BOOLEAN NOT NULL DEFAULT 0);
CREATE UNIQUE INDEX idx_users_username ON users(username);
CREATE UNIQUE INDEX idx_users_wallet_address ON users(wallet_address);
```

Auth/session tables:
- `auth_sessions`

```sql
CREATE TABLE auth_sessions (
            token TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            x_user_id TEXT,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id)
        );
```

- `auth_challenges`

```sql
CREATE TABLE auth_challenges (
            id TEXT PRIMARY KEY,
            wallet_address TEXT NOT NULL,
            challenge_text TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            consumed_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
```

- `social_peers`

```sql
CREATE TABLE social_peers (
            id TEXT PRIMARY KEY,
            platform TEXT NOT NULL,
            platform_user_id TEXT NOT NULL,
            platform_username TEXT,
            honcho_peer_id TEXT,
            user_id TEXT,
            interaction_count INTEGER NOT NULL DEFAULT 0,
            first_seen_at TEXT NOT NULL DEFAULT (datetime('now')),
            last_seen_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
CREATE UNIQUE INDEX idx_social_peers_platform_user
            ON social_peers(platform, platform_user_id);
CREATE INDEX idx_social_peers_username
            ON social_peers(platform, platform_username);
```

- `child_profiles`

```sql
CREATE TABLE child_profiles (
            id TEXT PRIMARY KEY,
            parent_wallet_address TEXT NOT NULL,
            derived_wallet_address TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            birthdate TEXT NOT NULL,
            emoji_pattern TEXT NOT NULL CHECK (json_valid(emoji_pattern)),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (parent_wallet_address) REFERENCES users(wallet_address)
        );
CREATE INDEX idx_child_profiles_parent_wallet
            ON child_profiles(parent_wallet_address);
```

### 6.2 `.anky` format

Defined in `src/models/anky_story.rs`.

Data model:
- `AnkyStoryMeta`
  - `anky_id`
  - optional `fid`
  - optional `cast_hash`
  - `written_at` (ISO 8601)
  - `duration_s`
  - `word_count`
  - `seed`
- `AnkyStoryPage`
  - optional `image_url`
  - `text: Vec<String>` paragraphs

Serialized shape:

```text
---
anky_id: <uuid>
fid: <optional farcaster fid>
cast_hash: <optional cast hash>
written_at: <iso-8601>
duration_s: 480
word_count: 612
seed: <seed-wallet-or-user>
---

:::page
image: <optional url>
paragraph one

paragraph two
:::
```

### 6.3 Where identities/sessions live

Web/browser identity:
- `users`
- `auth_sessions`
- Cookies: `anky_session`, `anky_user_id`
- `src/routes/auth.rs` handles X OAuth, Privy logout/verify, seed auth, and Farcaster auth

Mobile identity:
- `src/routes/swift.rs`
- Seed/EVM auth challenge/verify endpoints create or bind users by wallet address
- Child profiles live in `child_profiles`

Social identity/memory:
- `social_peers` maps external platform identities (`x`, `farcaster`) to Honcho peer ids and optional local `user_id`

### 6.4 On-chain state

Present and active:
- Base / EVM minting flow in `src/routes/swift.rs` via `prepare_mint`, `confirm_mint`, and `anky_metadata`
- Contract in repo: `contracts/AnkyMirrors.sol`
  - ERC-721
  - 4444 max supply
  - 1 USDC per mint
  - one mint per Farcaster FID
  - backend-signed EIP-712 mint payloads
- DB columns supporting mint state on `ankys`: `gas_funded_at`, `session_cid`, `metadata_uri`, `token_id`

Present but not part of the core writing-session state machine:
- Solana appears in marketing pages, DCA dashboard content, and static docs/assets.
- The core live write -> reflection -> image -> storage path is not storing session state on Solana.

## 7. Frontend Architecture

### 7.1 Stack

Main web frontend stack:
- Server-rendered Tera templates
- Plain HTML/CSS/vanilla JavaScript
- No repo-root Next.js/Vite/React frontend was found; only `docs/package.json` exists under `docs/`

Evidence:
- `src/routes/pages.rs` renders templates directly (`state.tera.render(...)`)
- `templates/home.html` contains the main session logic inline
- The browser writing flow uses `fetch(...)` and `EventSource(...)`, not a SPA framework router

### 7.2 Main writing interface

Primary writing UI:
- Route: `GET /write`
- File: `src/routes/pages.rs`
- Handler: `write_page`
- Template: `templates/home.html`

The actual core interaction logic lives inside `templates/home.html`:
- `beginSession()`
- `tick()`
- `pauseSession()`
- `doSend()`
- `sendToAnky(text)`
- `pollForAnkyPrompt(sessionId)`
- `streamReflection(ankyId)`

### 7.3 Backend communication mode

Browser writing UI communication pattern:
- REST `fetch` for checkpoints, warmup, and submission
- Polling `fetch` for `GET /api/writing/{sessionId}/status`
- SSE (`EventSource`) for `GET /api/stream-reflection/{ankyId}`

What it does **not** use for the core write loop:
- No WebSocket write stream in the browser flow
- No Redis-backed job orchestration in the live browser path

Other frontend channels elsewhere in the repo:
- WebSocket proxy for interview mode: `GET /ws/interview`
- Static served agent-skill assets: `/agent-skills/*`

### 7.4 Farcaster miniapp entry points

Yes, there is a Farcaster miniapp surface.

Observed entry points:
- `GET /.well-known/farcaster.json` -> `src/routes/mod.rs::farcaster_manifest`
  - Serves `static/farcaster.json`
- `POST /auth/farcaster/verify` -> `src/routes/auth.rs::farcaster_verify`
  - Trusts the FID from Farcaster MiniApp SDK context
- `GET /ankycoin` -> `src/routes/pages.rs::ankycoin_page`
  - Renders `templates/ankycoin_landing.html`
- `templates/ankycoin_landing.html`
  - Dynamically imports `@farcaster/miniapp-sdk`
  - Calls `sdk.actions.ready()`

Conclusion:
- The repo does have a Farcaster miniapp-facing frontend, but the main writing UI itself is still the server-rendered `/write` template rather than a dedicated Farcaster-only client app.

## 8. Failure Points & Concurrency

### 8.1 What happens with 3+ simultaneous writers?

Primary bottlenecks:
- SQLite is effectively single-lane.
  - `AppState.db` is `Arc<Mutex<rusqlite::Connection>>`.
  - Every checkpoint, final submission, SSE reflection save, webhook write, auth write, and background pipeline write serializes through one async mutex.
- GPU work is single-worker.
  - `gpu_job_worker` is one async loop in `src/main.rs`.
  - Pro/free channels change priority, not throughput.
  - Three simultaneous ankys will queue behind one another.
- Browser reflection streaming is not queued.
  - `api::stream_reflection` can start multiple concurrent upstream LLM calls at once.
  - If several ankys finish together, Anthropic/OpenRouter/Mind load spikes independently of the GPU queue.
- The in-memory GPU queue is unbounded.
  - A burst of ritual completions can accumulate indefinitely in memory.

Likely user-visible behavior under 3+ simultaneous writers:
- Checkpoints and final `POST /write` calls become latency-sensitive behind the SQLite mutex.
- Short writes may still finish quickly, but `anky_response`/`next_prompt` writes will lag.
- Full ankys will back up behind the single GPU worker.
- The UI can look inconsistent because reflection streaming and image generation are decoupled.

### 8.2 Is llama-server synchronous or queued?

Mixed answer:
- Browser reflection streaming: synchronous per request in `api::stream_reflection`.
- Many text calls use `services::mind::call(...)` directly in request/async-task context.
- There is **no** dedicated Redis-backed text-job queue in the active path.
- GPU/image work is queued, but only in the in-memory `GpuJobQueue`, not via Redis/Valkey.

### 8.3 Mutexes / locks

Locks present:
- Global async mutex over the single SQLite connection.
- `RwLock`s for live-state and GPU status.
- `Mutex<HashMap<...>>` caches/rate-limiters/session maps.

What is missing:
- No durable cross-process lock/lease for active browser writes.
- No durable queue/lock for GPU jobs.
- No durable lock for active agent chunk sessions.

### 8.4 Error handling / fallback gaps

Observed weak spots:
- Agent chunk sessions are in-memory only until finalization; process death loses them.
- Redis recovery exists in code, but the live GPU job path does not actually persist jobs into Redis.
- `generate_anky_from_writing` can exit early if required API keys are absent, leaving a pending/generating anky unless another path retries it.
- `services/comfyui.rs` hardcodes its base URL, which undermines the config layer.
- `voices.rs` has a whisper fallback to `localhost:8080/inference`, which conflicts with the installed llama-server service on the same port.
- There is no local systemd unit in `/etc/systemd/system` for the interview engine or TTS service despite code assuming they may exist.

### 8.5 If llama-server is down or overloaded

Actual behavior from source:
- `services::mind.rs` returns `AppError::Internal("Mind unavailable: ...")` / `Mind error ...`.
- Many higher-level generators catch this and fall back to Claude, then OpenRouter.
- `api::stream_reflection` itself streams Claude first and then falls back again if needed.

Practical consequence:
- The app degrades to external APIs rather than hard failing, but only if the relevant API keys are present.
- If local Mind and external fallbacks are all unavailable, browser users will see missing/failed reflection text and pending ankys can stall.

### 8.6 If ComfyUI is busy or down

Behavior from source:
- Main anky images try Gemini first and only use ComfyUI as fallback.
- Story/cuentacuentos images rely on ComfyUI workflows.
- ComfyUI polling waits up to 240 seconds for completion.

Consequence:
- Full ankys may still succeed without ComfyUI if Gemini image generation works.
- Story image flows are more fragile.
- Because the GPU queue is serial, a slow/busy ComfyUI path blocks later queued jobs behind it.

## 9. Configuration

### 9.1 Main config files / surfaces

| File | Purpose |
| --- | --- |
| `.env` | Local runtime secrets and overrides loaded by `dotenvy` and systemd `EnvironmentFile=`. |
| `.env.example` | Partial example env template; not fully synchronized with all vars used in code. |
| `src/config.rs` | Canonical Rust config loader and defaults. |
| `Cargo.toml` | Rust crate/dependency configuration. |
| `deploy/anky-mind.service` | Repo template for local llama-server. |
| `deploy/anky-heart.service` | Repo template for local ComfyUI service. |
| `/etc/systemd/system/anky.service` | Installed production-ish unit for the main Axum app on this workstation. |
| `/etc/systemd/system/anky-mind.service` | Installed llama-server unit on this workstation. |
| `/etc/systemd/system/anky-heart.service` | Installed ComfyUI unit on this workstation. |
| `/etc/valkey/valkey.conf` | Valkey runtime config used by the packaged service. |
| `static/farcaster.json` | Farcaster domain association / miniapp manifest payload served at `/.well-known/farcaster.json`. |

Secret-handling note:
- This audit intentionally does not copy values from `.env`.

### 9.2 Environment variables and purposes

| Env Var | Purpose |
| --- | --- |
| `PORT` | Axum listen port for the main Anky server. |
| `OLLAMA_BASE_URL` | Base URL for Ollama fallback inference. |
| `OLLAMA_MODEL` | Default Ollama model name. |
| `OLLAMA_LIGHT_MODEL` | Lightweight Ollama model override. |
| `OPENROUTER_API_KEY` | OpenRouter fallback API key. |
| `OPENROUTER_LIGHT_MODEL` | OpenRouter lightweight fallback model. |
| `ANTHROPIC_API_KEY` | Claude generation and streaming reflection key. |
| `GEMINI_API_KEY` | Gemini text/image generation key. |
| `BASE_RPC_URL` | Base chain RPC endpoint for mint/payment flows. |
| `USDC_ADDRESS` | USDC contract address used in Base payment flows. |
| `TREASURY_ADDRESS` | Treasury wallet for paid generation and mint flows. |
| `X402_FACILITATOR_URL` | x402 facilitator endpoint for paid API routes. |
| `TWITTER_CLIENT_ID` | X OAuth client id for web login. |
| `TWITTER_CLIENT_SECRET` | X OAuth client secret for web login. |
| `TWITTER_CALLBACK_URL` | OAuth callback URL for X login. |
| `X_BEARER_TOKEN` | X bot bearer token. |
| `X_CONSUMER_KEY` | X bot API key. |
| `X_CONSUMER_SECRET` | X bot API secret. |
| `X_ACCESS_TOKEN` | X bot access token. |
| `X_ACCESS_TOKEN_SECRET` | X bot access secret. |
| `TWITTER_BOT_USER_ID` | Expected X bot account id. |
| `PRIVY_APP_ID` | Privy application id for wallet-based auth. |
| `PRIVY_APP_SECRET` | Privy application secret. |
| `PRIVY_VERIFICATION_KEY` | Privy verification public key / PEM material. |
| `PUMPFUN_RTMP_URL` | RTMP ingest URL for livestream features. |
| `PUMPFUN_STREAM_KEY` | Pump.fun stream key. |
| `OPENAI_API_KEY` | OpenAI embeddings / auxiliary generation key. |
| `NEYNAR_API_KEY` | Neynar API key for Farcaster identity and webhooks. |
| `NEYNAR_SIGNER_UUID` | Neynar signer id for Farcaster posting. |
| `NEYNAR_WEBHOOK_SECRET` | Secret for verifying Farcaster webhook payloads. |
| `FARCASTER_BOT_FID` | Bot FID used by Farcaster integrations. |
| `XAI_API_KEY` | xAI key for video-related generation. |
| `CLOUDFLARE_API_TOKEN` | Cloudflare API token for cache purge / infra actions. |
| `CLOUDFLARE_ZONE_ID` | Cloudflare zone id. |
| `TRAINING_SECRET` | Secret for training live monitor endpoints. |
| `DATASET_PASSWORD` | Password gate for dataset gallery routes. |
| `COMFYUI_URL` | Configured ComfyUI base URL; not fully honored by `services/comfyui.rs`. |
| `HONCHO_API_KEY` | Honcho API key for identity/memory context. |
| `HONCHO_WORKSPACE_ID` | Honcho workspace id. |
| `HONCHO_BASE_URL` | Honcho API base URL. |
| `TTS_BASE_URL` | Base URL for the local F5-TTS service. |
| `R2_ACCOUNT_ID` | Cloudflare R2 account id. |
| `R2_BUCKET_NAME` | Cloudflare R2 bucket name. |
| `R2_ACCESS_KEY_ID` | R2 access key id. |
| `R2_SECRET_ACCESS_KEY` | R2 secret access key. |
| `R2_PUBLIC_URL` | Public base URL for uploaded R2 assets. |
| `FLUX_API_KEY` | Flux API key for image generation helpers. |
| `FLUX_SECRET_KEY` | Flux API secret. |
| `PINATA_JWT` | Pinata JWT for IPFS pinning during minting. |
| `ANKY_WALLET_PRIVATE_KEY` | Backend signer key for EIP-712 mint payloads. |
| `MIND_URL` | Base URL for the local llama-server / Mind endpoint. |
| `REDIS_URL` | Valkey/Redis connection string. |
| `APNS_KEY_PATH` | APNs signing key path. |
| `APNS_KEY_ID` | APNs key id. |
| `APNS_TEAM_ID` | APNs Apple team id. |
| `APNS_BUNDLE_ID` | APNs bundle id. |
| `APNS_ENVIRONMENT` | APNs environment (`production` or `sandbox`). |
| `BASE_DATASET_DIR` | Training orchestrator dataset root, read outside `Config`. |
| `COMFYUI_LORA_MODEL` | Optional override for the Flux LoRA filename. |

### 9.3 Extra direct env reads outside `Config`

- `ANKY_AGENT_API_KEY`: referenced in `scripts/autonomous_anky.py`, `scripts/test_session_api.py`
- `HF_TOKEN`: referenced in `scripts/export_round_two_dataset.py`
- `INSTAGRAM_ACCESS_TOKEN`: referenced in `scripts/autonomous_agent_v2.py`, `scripts/autonomous_anky.py`, `scripts/generate_anky_day2.py`
- `MODEL`: referenced in `agent-skills/anky/scripts/anky_session.py`
- `OPENAI_BASE_URL`: referenced in `agent-skills/anky/scripts/anky_session.py`, `agent-skills/anky/scripts/anky_server.py`
- `OPENAI_MODEL`: referenced in `agent-skills/anky/scripts/anky_session.py`, `agent-skills/anky/scripts/anky_server.py`
- `TWITTER_BEARER_TOKEN`: referenced in `scripts/generate_anky_day2.py`

### 9.4 Hardcoded values that should be configurable

High-signal hardcoded values found in source:
- `src/services/comfyui.rs`
  - hardcoded `COMFYUI_URL = http://127.0.0.1:8188`
  - hardcoded model filenames: `flux1-dev.safetensors`, `ae.safetensors`, `clip_l.safetensors`, `t5xxl_fp8_e4m3fn.safetensors`
  - hardcoded LoRA directory: `/home/kithkui/ComfyUI/models/loras`
- `src/services/hermes.rs`
  - hardcoded bridge URL `http://127.0.0.1:8891`
- `src/routes/voices.rs`
  - hardcoded whisper fallback `http://localhost:8080/inference`
- `src/routes/session.rs`
  - hardcoded `CHUNK_TIMEOUT_SECS = 8`, `MAX_WORDS_PER_CHUNK = 50`, `ANKY_THRESHOLD_SECS = 480.0`
- `templates/home.html`
  - hardcoded browser constants `IDLE_TIMEOUT = 8.0`, `SESSION_DURATION = 480.0`, `CHECKPOINT_INTERVAL = 30000`
- `src/main.rs`
  - single GPU worker task; concurrency is implicit and not configurable from env

## 10. What's Dead

### 10.1 Likely abandoned / partially wired features

1. Redis-backed job queue is mostly dormant.
- `src/services/redis_queue.rs` defines persistent queues and crash recovery.
- `src/services/mod.rs` marks it `#[allow(dead_code)]`.
- The active GPU pipeline uses the in-memory `GpuJobQueue` in `src/state.rs` instead.
- Current Valkey had no `anky:*` keys during inspection.

2. Live streaming comments and wiring disagree.
- `src/routes/mod.rs` says `live.rs` routes are "disabled" and "not wired up".
- But `GET /api/ankys/today` and `GET /api/live-status` are still routed.

3. Repo-vs-machine service drift.
- `deploy/anky-mind.service` no longer matches `/etc/systemd/system/anky-mind.service`.
- Anyone trusting the repo unit file will have the wrong `--parallel` and `--ctx-size` settings for the actual workstation.

4. README is stale relative to the code.
- `README.md` still describes meditation/breathwork/facilitator flows as core parts of the practice loop.
- The current code path is centered on writing, reflection, image generation, mobile APIs, and minting.
- README still frames Ollama as a main local dependency while the active installed local text stack is `llama-server` via Mind and `ollama.service` is disabled.

5. Interview engine deployment is incomplete in-repo.
- `interview-engine/server.py` exists and `GET /ws/interview` proxies to it.
- No matching `/etc/systemd/system/interview*.service` unit was found on this workstation.

6. TTS service dependency is assumed, not provisioned here.
- `src/services/tts.rs` and voice routes expect `TTS_BASE_URL`.
- No corresponding local systemd unit was found during this audit.

7. Whisper fallback is likely broken in the current workstation topology.
- `src/routes/voices.rs` assumes whisper.cpp on `localhost:8080/inference`.
- Installed `anky-mind.service` already owns `127.0.0.1:8080`.

### 10.2 TODO / FIXME / HACK markers actually found

`rg -n --glob '!static/ethers.umd.min.js' "TODO|FIXME|HACK" ...` only surfaced two live TODOs in app code:
- `src/services/notification.rs:11` — TODO: integrate an actual email service
- `src/services/notification.rs:26` — TODO: integrate Telegram Bot API

### 10.3 Architectural dead ends / risks worth treating as debt

- Single-connection SQLite behind a global mutex is the main concurrency ceiling.
- In-memory agent sessions and in-memory GPU queues mean process restarts still lose meaningful live state despite Redis recovery code existing.
- The browser write flow, mobile flow, and agent chunked flow have diverged enough that their post-write behavior is no longer obviously identical.
- The Farcaster/miniapp surface, Ankycoin surface, and core `/write` surface now coexist, but they are not clearly separated into bounded frontend packages or deployment units.

## Bottom Line

The repo is one Axum monolith with three distinct write entry points:
- browser template flow (`/write` + `templates/home.html`)
- mobile REST flow (`/swift/v1|v2/write`)
- agent chunk flow (`/api/v1/session/*`)

The critical live-writing mechanic is enforced in two places:
- browser-side in `templates/home.html`
- server-side for agent chunk sessions in `src/routes/session.rs`

Persistence is centered on SQLite, but durable queueing is not. The most important architectural reality for future decisions is that the repo already behaves like a multi-surface product while still relying on a single-process, single-DB-connection, single-GPU-worker runtime model.
