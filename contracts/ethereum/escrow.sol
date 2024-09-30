// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.26;

import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

/**
 * @dev Interface for the ICS20 transfer bank to facilitate cross-chain token transfers.
 */
interface IICS20TransferBank {
    function sendTransfer(
        string calldata denom,
        uint256 amount,
        string calldata receiver,
        string calldata sourcePort,
        string calldata sourceChannel,
        uint64 timeoutHeight,
        uint64 timeoutTimestamp,
        string calldata memo
    ) external payable;
}

/**
 * @dev Struct representing a Solver's transfer.
 * Contains data for transferring tokens from a solver to a user.
 */
struct SolverTransfer {
    uint intentId;             // Unique identifier for the intent
    address tokenOut;          // Address of the token to be transferred
    uint256 amountOut;         // Amount of tokens to be transferred
    address dstUser;           // Destination user for the transfer
    bool singleDomain;         // Boolean flag indicating if the transfer is within a single domain
    string solverOut;          // Address solver
}

/**
 * @dev Struct representing the parameters for the hop transfer.
 * Used for cross-chain or multiple hop transfers.
 */
struct HopParams {
    string sourcePort;         // The source port for the hop
    string sourceChannel;      // The source channel for the hop
    string denomTimeout;       // Denomination and timeout information
    string receiver;           // Receiver address for the hop transfer
}

/**
 * @dev Struct representing detailed information about an intent.
 * Includes both input and output tokens, user addresses, and transfer details.
 */
struct IntentInfo {
    address tokenIn;           // Input token address
    uint256 amountIn;          // Amount of input tokens
    address srcUser;           // Source user address
    string tokenOut;           // Output token (denomination) as a string
    uint256 amountOut;          // Amount of output tokens
    string dstUser;            // Destination user address
    string winnerSolver;       // Address of the winning solver
    uint256 timeout;           // Timeout for the intent
}

/**
 * @dev Struct representing transfer data for tokens.
 */
struct TransferData {
    string denom;              // Denomination of the token
    string sender;             // Address of the sender
    string receiver;           // Address of the receiver
    uint256 amount;            // Amount of tokens to be transferred
    string memo;               // Memo or metadata associated with the transfer
}

/**
 * @dev Struct representing the height of a packet for cross-chain transfers.
 */
struct Height {
    uint64 revision_number;    // Revision number for the block
    uint64 revision_height;    // Revision height for the block
}

/**
 * @dev Struct representing data of a packet for cross-chain communication.
 */
struct PacketData {
    uint64 sequence;                  // Sequence number of the packet
    string source_port;               // Source port in the cross-chain communication
    string source_channel;            // Source channel in the cross-chain communication
    string destination_port;          // Destination port in the cross-chain communication
    string destination_channel;       // Destination channel in the cross-chain communication
    bytes data;                       // Data contained in the packet
    Height timeout_height;            // Timeout height for the packet
    uint64 timeout_timestamp;         // Timeout timestamp for the packet
}

/**
 * @dev The Escrow contract manages funds and cross-chain token transfers.
 * It supports transferring tokens between users and across different chains.
 */
contract Escrow {
    using SafeERC20 for IERC20;

    // Events emitted for cross-chain communication and state updates
    event CrossChainMsgSolver(uint intentId, string winnerSolver, string token, address user, uint amount, string solverOut);
    event CrossChainMsgUser(uint intentId, address user);
    event FundsEscrowed(uint intentId);

    // Constant variables for demo purposes
    string public constant DUMMY = "3dsg7C6LGnCL17i4a4qU5N3n6RsKR7AFpVUyRc1hToAQ";
    address public constant BRIDGE_CONTRACT = 0x148ACD3Cd4D6A17CD2AbbEcD0745b09B62c64f84;

    // Contract state variables
    address public owner;                            // Owner of the contract
    IICS20TransferBank public ics20TransferBank;     // ICS20 transfer bank instance for cross-chain transfers
    HopParams public picasso_params;                 // Parameters for Picasso hop
    HopParams public next_hop_params;                // Parameters for the next hop transfer
    mapping(uint => IntentInfo) public intents;      // Mapping to store intents by their ID
    uint nextIntentId;

    /**
     * @dev Constructor to initialize the Escrow contract with specified parameters.
     * @param _ics20TransferBank Address of the ICS20 transfer bank contract.
     * @param _picasso_params Parameters for the Picasso hop transfer.
     * @param _next_hop_params Parameters for the next hop transfer.
     */
    // [BRIDGE_CONTRACT]
    // ["transfer","channel-2","0x36dd1bfe89d409f869fabbe72c3cf72ea8b460f6","pfm"]
    // ["transfer","channel-71","600000000000000","yAJJJMZmjWSQjvq8WuARKygH8KJkeQTXB5BGJBJcR4T"]
    // approve token to transfer from solver to address(this)
    // send token_bridge to address(this) and approve 0x5933fde9fa60d4f1c0124aa7a7a988f46ba42d78 from address(this)
    constructor(address _ics20TransferBank, HopParams memory _picasso_params, HopParams memory _next_hop_params) {
        owner = msg.sender;
        ics20TransferBank = IICS20TransferBank(_ics20TransferBank);
        picasso_params = _picasso_params;
        next_hop_params = _next_hop_params;
    }

    /**
    * @dev Handles the receipt of a cross-chain transfer, processes the transfer data and packet data, and handles the funds based on the memo content.
    * Depending on the memo, it either processes a user withdrawal or handles the transfer to a solver.
    *
    * @param transferData The data associated with the transfer, including the denomination and memo.
    * @param packetData The data associated with the packet sent in the cross-chain message.
    * @return A boolean indicating if the transfer processing was successful.
    */
    function onReceiveTransfer(
        TransferData calldata transferData,
        PacketData calldata packetData
    ) external returns (bool) {
        require(msg.sender == BRIDGE_CONTRACT, "msg.sender != BRIDGE_CONTRACT");
        string memory dummy = extractString(transferData.denom);
        require(keccak256(abi.encodePacked(dummy)) == keccak256(abi.encodePacked(DUMMY)), "denom doesn't have DUMMY token");

        (
            bool withdraw_user,
            string memory intentId,
            string memory from,
            string memory token,
            string memory to,
            string memory amount,
            string memory solver_out
        ) = splitMemo(transferData.memo);

        IntentInfo memory intent = intents[parseUint(intentId)];
        require(intent.srcUser != address(0), "intent doesn't exist");

        if (withdraw_user) {
            require(keccak256(abi.encodePacked(intent.dstUser)) == keccak256(abi.encodePacked(from)), "intent.dstUser != from");
            IERC20(intent.tokenIn).safeTransfer(intent.srcUser, intent.amountIn);
        }
        else {
            require(keccak256(abi.encodePacked(intent.winnerSolver)) == keccak256(abi.encodePacked(from)), "intent.dstUser != from");
            require(keccak256(abi.encodePacked(intent.tokenOut)) == keccak256(abi.encodePacked(token)), "intent.tokenOut != token");
            require(keccak256(abi.encodePacked(intent.dstUser)) == keccak256(abi.encodePacked(to)), "intent.dstUser != to");
            require(parseUint(intent.amountOut) <= parseUint(amount), "intent.amountOut > amount");
            IERC20(intent.tokenIn).safeTransfer(parseAddress(solver_out), intent.amountIn);
        }

        return true;
    }

    /**
     * @dev Function to escrow funds for a user based on an intent.
     * The user must approve the token transfer to the Escrow contract.
     * @param newIntentInfo Struct containing details of the intent.
     */
    // ["0x39F98f32eb5fe4C568c7252e45fd48f8DC415d8e","1000","0x25967E0621288bc958DC282c0CA6F451b17aef1c","0x39F98f32eb5fe4C568c7252e45fd48f8DC415d8e","500","0x39F98f32eb5fe4C568c7252e45fd48f8DC415d8e","","3600"]
    function escrowFunds(
        IntentInfo calldata newIntentInfo
    ) public payable returns (uint) {
        uint intentId = nextIntentId++;
        IntentInfo memory intent = intents[intentId];
        require(intent.srcUser == address(0), "intent already exist");
        require(bytes(newIntentInfo.winnerSolver).length == 0, "winnerSolver must be empty string");
        require(newIntentInfo.srcUser == msg.sender, "newIntentInfo.src_user != msg.sender");

        IERC20(newIntentInfo.tokenIn).safeTransferFrom(msg.sender, address(this), newIntentInfo.amountIn);
        intents[intentId] = newIntentInfo;

        emit FundsEscrowed(intentId);
        return intentId;
    }

    /**
     * @dev Update auction data for a specific intent.
     * @param intentId Unique identifier for the intent.
     * @param amountOut The amount of tokens to be transferred out.
     * @param winnerSolver Address of the winning solver.
     */
    function updateAuctionData(
        uint intentId,
        uint amountOut,
        string calldata winnerSolver
    ) public onlyOwner {
        IntentInfo storage intent = intents[intentId];
        intent.amountOut = amountOut;
        intent.winnerSolver = winnerSolver;
    }

    /**
     * @dev Send funds to a user based on the provided solver transfer data.
     * Handles both single domain and cross-chain transfers.
     * @param solverTransferData The data related to the solver's transfer.
     */
    function sendFundsToUser(
        SolverTransfer calldata solverTransferData
    ) public payable {
        // Execute solver transfer to user
        IERC20(solverTransferData.tokenOut).safeTransferFrom(msg.sender, solverTransferData.dstUser, solverTransferData.amountOut);

        if (solverTransferData.singleDomain) {
            // Single domain transfer: execute transfer from user to solver
            IntentInfo memory intent = intents[solverTransferData.intentId];
            require(intent.srcUser != address(0), "intent doesn't exist");
            require(parseAddress(intent.winnerSolver) == msg.sender, "intent.winnerSolver != msg.sender");

            IERC20(intent.tokenIn).safeTransfer(msg.sender, intent.amountIn);
            delete intents[solverTransferData.intentId];
        } else {
            // Execute Bridge Transfer
            /*ics20TransferBank.sendTransfer{value: msg.value}(
                picasso_params.denomTimeout,
                1000,
                picasso_params.receiver,
                picasso_params.sourcePort,
                picasso_params.sourceChannel,
                1000000000000000,
                10000000000000000000,
                constructJson(
                    next_hop_params,
                    intent.tokenOut,
                    intent.amountOut,
                    solverTransferData.solverOut
                )
            );*/
            emit CrossChainMsgSolver(
                solverTransferData.intentId,
                addressToString(msg.sender),
                addressToString(solverTransferData.tokenOut),
                solverTransferData.dstUser,
                solverTransferData.amountOut,
                solverTransferData.solverOut
            );
        }
    }

    /**
     * @dev Allows a user to cancel an intent and retrieve their funds.
     * @param intentId Unique identifier for the intent.
     * @param singleDomain Boolean flag indicating if the intent is within a single domain.
     */
    function userCancelIntent(
        uint intentId,
        bool singleDomain
    ) external {
        if (singleDomain) {
            // Single domain transfer cancellation
            IntentInfo memory intent = intents[intentId];
            require(intent.srcUser == msg.sender, "intent.srcUser != msg.sender");
            require(intent.timeout < block.timestamp, "intent.timeout > block.timestamp");

            IERC20(intent.tokenIn).safeTransfer(msg.sender, intent.amountIn);
            delete intents[intentId];
        } else {
            /*ics20TransferBank.sendTransfer{value: msg.value}(
                ...
            );*/
            emit CrossChainMsgUser(intentId, msg.sender);
        }
    }

    /**
    * @dev Splits the memo string into individual components based on the format.
    * The memo format can be one of two formats:
    * 1. Short format: 'false,intentId,from'
    * 2. Full format: 'true,intentId,from,token,to,amount'
    *
    * @param memo The full memo string with the bool as the first part, followed by the respective fields.
    * @return isFullFormat A boolean indicating if the memo is in the full format or the short format.
    * @return intentId The intent identifier extracted from the memo.
    * @return from The "from" address extracted from the memo.
    * @return token The token address (only in full format, empty in short format).
    * @return to The "to" address (only in full format, empty in short format).
    * @return amount The amount of tokens (only in full format, empty in short format).
    */
    function splitMemo(string calldata memo) internal pure returns (
        bool isFullFormat,
        string memory intentId,
        string memory from,
        string memory token,
        string memory to,
        string memory amount,
        string memory solver_out
    ) {
        // Split the memo string into parts by commas
        string[] memory parts = splitStringByDelimiter(memo, ",");

        // Extract the bool from the first part of the string
        isFullFormat = (keccak256(bytes(parts[0])) == keccak256(bytes("true")));

        if (isFullFormat) {
            // Full format requires exactly 6 parts
            require(parts.length == 6, "Invalid full format");

            // Assign the fields in the full format
            intentId = parts[1];
            from = parts[2];
            token = parts[3];
            to = parts[4];
            amount = parts[5];
            solver_out = parts[6];
        } else {
            // Short format requires exactly 3 parts
            require(parts.length == 3, "Invalid short format");

            // Assign the fields in the short format
            intentId = parts[1];
            from = parts[2];

            // Leave token, to, and amount as empty strings
            token = "";
            to = "";
            amount = "";
        }
    }

    /**
    * @dev Splits a string into an array of substrings based on a delimiter.
    *
    * @param str The input string to be split.
    * @param delimiter The delimiter used to split the string.
    * @return An array of substrings split by the delimiter.
    */
    function splitStringByDelimiter(string memory str, string memory delimiter) internal pure returns (string[] memory) {
        bytes memory strBytes = bytes(str);
        bytes memory delimiterBytes = bytes(delimiter);
        uint256 delimiterLength = delimiterBytes.length;

        // Calculate how many substrings we will have
        uint256 count = 1;
        for (uint256 i = 0; i < strBytes.length - delimiterLength + 1; i++) {
            bool _match = true;
            for (uint256 j = 0; j < delimiterLength; j++) {
                if (strBytes[i + j] != delimiterBytes[j]) {
                    _match = false;
                    break;
                }
            }
            if (_match) {
                count++;
                i += delimiterLength - 1;
            }
        }

        // Split the string into parts
        string[] memory result = new string[](count);
        uint256 start = 0;
        uint256 resultIndex = 0;
        for (uint256 i = 0; i < strBytes.length - delimiterLength + 1; i++) {
            bool _match = true;
            for (uint256 j = 0; j < delimiterLength; j++) {
                if (strBytes[i + j] != delimiterBytes[j]) {
                    _match = false;
                    break;
                }
            }
            if (_match) {
                result[resultIndex++] = substring(str, start, i);
                start = i + delimiterLength;
                i += delimiterLength - 1;
            }
        }
        result[resultIndex] = substring(str, start, strBytes.length);

        return result;
    }

    /**
    * @dev Extracts a substring from a string.
    *
    * @param str The input string from which the substring will be extracted.
    * @param startIndex The start index of the substring (inclusive).
    * @param endIndex The end index of the substring (exclusive).
    * @return The extracted substring.
    */
    function substring(string memory str, uint256 startIndex, uint256 endIndex) internal pure returns (string memory) {
        bytes memory strBytes = bytes(str);
        bytes memory result = new bytes(endIndex - startIndex);
        for (uint256 i = startIndex; i < endIndex; i++) {
            result[i - startIndex] = strBytes[i];
        }
        return string(result);
    }


    /**
     * @dev Parse a string into an address.
     * @param str The string to parse.
     * @return The parsed address.
     */
    function parseAddress(string memory str) internal pure returns (address) {
        bytes memory tmp = bytes(str);
        uint160 addr = 0;
        for (uint160 i = 2; i < 42; i++) {
            uint160 b = uint160(uint8(tmp[i]));
            if (b >= 48 && b <= 57) {
                addr = addr * 16 + (b - 48);
            } else if (b >= 97 && b <= 102) {
                addr = addr * 16 + (b - 87);
            } else if (b >= 65 && b <= 70) {
                addr = addr * 16 + (b - 55);
            }
        }
        return address(addr);
    }

    /**
     * @dev Parse a string into a uint256.
     * @param str The string to parse.
     * @return The parsed uint256.
     */
    function parseUint(string memory str) internal pure returns (uint256) {
        bytes memory b = bytes(str);
        uint256 result = 0;
        for (uint256 i = 0; i < b.length; i++) {
            if (uint8(b[i]) >= 48 && uint8(b[i]) <= 57) {
                result = result * 10 + (uint8(b[i]) - 48);
            }
        }
        return result;
    }

    /**
     * @dev Extract a substring from the input string.
     * @param input The input string.
     * @return The extracted substring.
     */
    function extractString(string memory input) internal pure returns (string memory) {
        bytes memory inputBytes = bytes(input);
        uint start;
        uint end = inputBytes.length;

        // Find the start of the substring
        for (uint i = 0; i < inputBytes.length; i++) {
            if (inputBytes[i] == "/") {
                start = i + 1;
            }
        }

        // Extract the substring
        bytes memory result = new bytes(end - start);
        for (uint i = start; i < end; i++) {
            result[i - start] = inputBytes[i];
        }

        return string(result);
    }

    // Helper function to convert an address to a string
    function addressToString(address _addr) public pure returns (string memory) {
        bytes32 value = bytes32(uint256(uint160(_addr)));
        bytes memory alphabet = "0123456789abcdef";

        bytes memory str = new bytes(42);
        str[0] = '0';
        str[1] = 'x';
        for (uint256 i = 0; i < 20; i++) {
            str[2 + i * 2] = alphabet[uint8(value[i + 12] >> 4)];
            str[3 + i * 2] = alphabet[uint8(value[i + 12] & 0x0f)];
        }
        return string(str);
    }

    // Helper function to convert uint256 to a string
    function uintToString(uint256 _i) public pure returns (string memory) {
        if (_i == 0) {
            return "0";
        }
        uint256 j = _i;
        uint256 length;
        while (j != 0) {
            length++;
            j /= 10;
        }
        bytes memory bstr = new bytes(length);
        uint256 k = length;
        while (_i != 0) {
            k = k - 1;
            uint8 temp = (48 + uint8(_i - (_i / 10) * 10));
            bstr[k] = bytes1(temp);
            _i /= 10;
        }
        return string(bstr);
    }

    /**
     * @dev Execute a call to a target address with the provided data.
     * @param target The target address.
     * @param data The data to call the target with.
     * @return The result of the call.
     */
    function executeCall(
        address target,
        bytes calldata data
    ) public onlyOwner returns (bytes memory) {
        (bool success, bytes memory result) = target.call(data);
        require(success, "call failed!");
        return result;
    }

    /**
     * @dev Change the owner of the contract.
     * @param _owner The new owner address.
     */
    function changeOwner(address _owner) public onlyOwner {
        owner = _owner;
    }

    /**
     * @dev Change the Picasso hop parameters.
     * @param _picasso_params The new Picasso hop parameters.
     */
    function changePicassoParams(HopParams calldata _picasso_params) public onlyOwner {
        picasso_params = _picasso_params;
    }

    /**
     * @dev Change the next hop parameters.
     * @param _next_hop_params The new next hop parameters.
     */
    function changeNextHopParams(HopParams calldata _next_hop_params) public onlyOwner {
        next_hop_params = _next_hop_params;
    }

    /**
     * @dev Change the ICS20 transfer bank address.
     * @param _ics20TransferBank The new ICS20 transfer bank address.
     */
    function changeIcs20TransferBank(address _ics20TransferBank) public onlyOwner {
        ics20TransferBank = IICS20TransferBank(_ics20TransferBank);
    }

    function getIntentInfo(uint intentId) view public returns(IntentInfo memory) {
        return intents[intentId];
    }

    /**
     * @dev Modifier to restrict function access to only the owner.
     */
    modifier onlyOwner() {
        require(msg.sender == owner, "Only the owner can call this function");
        _;
    }

    // Fallback function to receive Ether
    fallback() external payable {}

    // Receive function to receive Ether
    receive() external payable {}
}
