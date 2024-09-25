// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.25;

import "contracts/ethereum/SafeERC20.sol";

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
    string intentId;           // Unique identifier for the intent
    address tokenOut;          // Address of the token to be transferred
    uint256 amountOut;         // Amount of tokens to be transferred
    address dstUser;           // Destination user for the transfer
    bool singleDomain;         // Boolean flag indicating if the transfer is within a single domain
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
    string amountOut;          // Amount of output tokens
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
    event CrossChainMsgSolver(string intentId, string winnerSolver, string token, string user, string amount);
    event CrossChainMsgUser(string intentId, string user);
    event OnReceiveMsg(string token, string amount, string to);
    event FundsEscrowed(IntentInfo intent);

    // Constant variables for demo purposes
    string public constant DUMMY = "3dsg7C6LGnCL17i4a4qU5N3n6RsKR7AFpVUyRc1hToAQ";
    address public constant BRIDGE_CONTRACT = 0x148ACD3Cd4D6A17CD2AbbEcD0745b09B62c64f84;

    // Contract state variables
    address public owner;                            // Owner of the contract
    IICS20TransferBank public ics20TransferBank;     // ICS20 transfer bank instance for cross-chain transfers
    HopParams public picasso_params;                 // Parameters for Picasso hop
    HopParams public next_hop_params;                // Parameters for the next hop transfer
    mapping(string => IntentInfo) public intents;    // Mapping to store intents by their ID

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

    /*function onReceiveTransfer(
        TransferData memory transferData,
        PacketData calldata packetData
    ) external view returns (bool) {
        require(msg.sender == BRIDGE_CONTRACT, "msg.sender != BRIDGE_CONTRACT");
        string memory dummy = extractString(transferData.denom);
        require(keccak256(abi.encodePacked(dummy)) == keccak256(abi.encodePacked(DUMMY)), "denom doesn't have DUMMY token");

        (
            string memory intentId,
            string memory winnerSolver,
            string memory token,
            string memory user,
            string memory amount
        ) = splitMemo(transferData.memo);



        return true;
    }*/

    /**
     * @dev Function to escrow funds for a user based on an intent.
     * The user must approve the token transfer to the Escrow contract.
     * @param intentId Unique identifier for the intent.
     * @param newIntentInfo Struct containing details of the intent.
     */
    function escrowFunds(
        string calldata intentId,
        IntentInfo calldata newIntentInfo
    ) public onlyOwner {
        IntentInfo memory intent = intents[intentId];
        require(intent.srcUser == address(0), "intent already exist");
        require(bytes(newIntentInfo.winnerSolver).length == 0, "winnerSolver must be empty string");
        require(newIntentInfo.srcUser == msg.sender, "newIntentInfo.src_user != msg.sender");

        IERC20(newIntentInfo.tokenIn).safeTransferFrom(msg.sender, address(this), newIntentInfo.amountIn);
        intents[intentId] = newIntentInfo;

        emit FundsEscrowed(intent);
    }

    /**
     * @dev Update auction data for a specific intent.
     * @param intentId Unique identifier for the intent.
     * @param amountOut The amount of tokens to be transferred out.
     * @param winnerSolver Address of the winning solver.
     */
    function updateAuctionData(
        string calldata intentId,
        string calldata amountOut,
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
                addressToString(solverTransferData.dstUser),
                uintToString(solverTransferData.amountOut)
            );
        }
    }

    /**
     * @dev Allows a user to cancel an intent and retrieve their funds.
     * @param intentId Unique identifier for the intent.
     * @param singleDomain Boolean flag indicating if the intent is within a single domain.
     */
    function userCancelIntent(
        string calldata intentId,
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
            emit CrossChainMsgUser(intentId, addressToString(msg.sender));
        }
    }

    function splitMemo(string memory memo) internal pure returns (
        string memory intentId,
        string memory winnerSolver,
        string memory token,
        string memory user,
        string memory amount
    ) {
        bytes memory memoBytes = bytes(memo);

        // Fixed lengths for intentId, winnerSolver, token, and user
        uint256 intentIdLength = 8;
        uint256 fixedLength = 42;

        // Ensure the memo is long enough
        require(memoBytes.length > intentIdLength + 3 * fixedLength, "Invalid input length");

        // Part 1: intentId (first 8 characters)
        intentId = substring(memo, 0, intentIdLength);

        // Part 2: winnerSolver (next 42 characters after intentId)
        winnerSolver = substring(memo, intentIdLength, intentIdLength + fixedLength);

        // Part 3: token (next 42 characters after winnerSolver)
        token = substring(memo, intentIdLength + fixedLength, intentIdLength + 2 * fixedLength);

        // Part 4: user (next 42 characters after token)
        user = substring(memo, intentIdLength + 2 * fixedLength, intentIdLength + 3 * fixedLength);

        // Find the last comma and extract the amount (last part after the final comma)
        uint256 lastComma = findLastComma(memo);
        require(lastComma != 0, "No comma found in the memo");

        // Part 5: amount (from last comma to the end of the string)
        amount = substring(memo, lastComma + 1, memoBytes.length);
    }

    // Helper function to extract a substring
    function substring(string memory str, uint startIndex, uint endIndex) internal pure returns (string memory) {
        bytes memory strBytes = bytes(str);
        bytes memory result = new bytes(endIndex - startIndex);

        for (uint i = startIndex; i < endIndex; i++) {
            result[i - startIndex] = strBytes[i];
        }

        return string(result);
    }

    // Helper function to find the position of the last comma in the string
    function findLastComma(string memory memo) internal pure returns (uint256) {
        bytes memory memoBytes = bytes(memo);

        for (uint256 i = memoBytes.length; i > 0; i--) {
            if (memoBytes[i - 1] == bytes1(",")) {
                return i - 1;  // Return the position of the last comma
            }
        }

        return 0;  // Return 0 if no comma is found
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

