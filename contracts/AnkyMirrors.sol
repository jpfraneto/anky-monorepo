// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";

/// @title AnkyMirrors — 4444 unique Anky mirror NFTs, 1 USDC each, 1 per FID.
/// @notice The backend signs a MirrorMint payload per FID. The user pays 1 USDC
///         and submits the signature to mint. The contract enforces 1-per-FID and
///         verifies the backend signer.
contract AnkyMirrors is ERC721, EIP712 {
    using ECDSA for bytes32;

    uint256 public constant MAX_SUPPLY = 4444;
    uint256 public constant PRICE = 1_000_000; // 1 USDC (6 decimals)

    IERC20 public immutable usdc;
    address public immutable signer;   // backend wallet that signs mint payloads
    address public immutable treasury; // where USDC goes

    uint256 public totalSupply;

    // fid => tokenId (0 means not minted)
    mapping(uint256 => uint256) public fidToToken;
    // tokenId => fid
    mapping(uint256 => uint256) public tokenToFid;
    // tokenId => metadata URI
    mapping(uint256 => string) private _tokenURIs;

    bytes32 private constant MINT_TYPEHASH =
        keccak256("MirrorMint(address minter,uint256 fid,string mirrorId,uint256 deadline)");

    error AlreadyMinted();
    error SoldOut();
    error BadSignature();
    error Expired();
    error TransferFailed();

    constructor(
        address _usdc,
        address _signer,
        address _treasury
    ) ERC721("Anky Mirrors", "ANKYMIRROR") EIP712("AnkyMirrors", "1") {
        usdc = IERC20(_usdc);
        signer = _signer;
        treasury = _treasury;
    }

    /// @notice Mint your Anky mirror. Requires 1 USDC approval + backend signature.
    /// @param fid       Your Farcaster ID (enforced 1-per-FID)
    /// @param mirrorId  The mirror UUID from the backend
    /// @param deadline  Signature expiry timestamp
    /// @param signature EIP-712 signature from the backend signer
    function mint(
        uint256 fid,
        string calldata mirrorId,
        uint256 deadline,
        bytes calldata signature
    ) external {
        if (block.timestamp > deadline) revert Expired();
        if (totalSupply >= MAX_SUPPLY) revert SoldOut();
        if (fidToToken[fid] != 0) revert AlreadyMinted();

        // Verify backend signature
        bytes32 structHash = keccak256(
            abi.encode(MINT_TYPEHASH, msg.sender, fid, keccak256(bytes(mirrorId)), deadline)
        );
        bytes32 digest = _hashTypedDataV4(structHash);
        if (digest.recover(signature) != signer) revert BadSignature();

        // Collect payment
        if (!usdc.transferFrom(msg.sender, treasury, PRICE)) revert TransferFailed();

        // Mint
        uint256 tokenId = ++totalSupply;
        fidToToken[fid] = tokenId;
        tokenToFid[tokenId] = fid;
        _tokenURIs[tokenId] = string.concat("https://ankycoin.com/api/mirror/metadata/", mirrorId);

        _safeMint(msg.sender, tokenId);
    }

    function tokenURI(uint256 tokenId) public view override returns (string memory) {
        _requireOwned(tokenId);
        return _tokenURIs[tokenId];
    }
}
